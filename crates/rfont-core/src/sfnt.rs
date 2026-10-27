//! SFNT container assembly shared by TTF/OTF output and WOFF/WOFF2 decoding.

use rfont_types::{
    FontError, SFNT_CHECKSUM_MAGIC, TABLE_DIR_ENTRY_SIZE, TableRecord, Tag, WriteBytes, Writer,
};
use std::collections::BTreeMap;

use crate::checksum::calc_sfnt_checksum;
use crate::pad4;

/// Offset of `checkSumAdjustment` inside the head table.
pub const HEAD_CHECKSUM_ADJUSTMENT_OFFSET: usize = 8;

/// Compute the binary-search fields of an SFNT offset table.
///
/// - `searchRange = 16 * 2^floor(log2(numTables))`
/// - `entrySelector = floor(log2(numTables))`
/// - `rangeShift = numTables * 16 - searchRange`
pub fn table_header_fields(num_tables: u16) -> (u16, u16, u16) {
    if num_tables == 0 {
        return (0, 0, 0);
    }

    let entry_selector = 15 - num_tables.leading_zeros();
    let max_pow2 = 1u32 << entry_selector;
    let search_range = max_pow2 * 16;
    let range_shift = (num_tables as u32 * 16).saturating_sub(search_range);

    (
        search_range as u16,
        entry_selector as u16,
        range_shift as u16,
    )
}

/// Collects tables and serializes them as a valid SFNT (TTF or OTF) file.
///
/// Tables are stored sorted by tag, table data is 4-byte aligned, every
/// directory checksum is computed, and `head.checkSumAdjustment` is patched.
#[derive(Debug, Clone)]
pub struct SfntRepack {
    flavor: u32,
    tables: BTreeMap<Tag, Vec<u8>>,
}

impl SfntRepack {
    pub fn new(flavor: u32) -> Self {
        Self {
            flavor,
            tables: BTreeMap::new(),
        }
    }

    /// Insert a table. Inserting the same tag twice keeps the last value.
    pub fn insert(&mut self, tag: Tag, data: Vec<u8>) {
        self.tables.insert(tag, data);
    }

    pub fn extend<I>(&mut self, tables: I)
    where
        I: IntoIterator<Item = (Tag, Vec<u8>)>,
    {
        for (tag, data) in tables {
            self.insert(tag, data);
        }
    }

    pub fn build(mut self) -> Result<Vec<u8>, FontError> {
        if self.tables.is_empty() {
            return Err(FontError::Generic("SFNT: no tables".to_string()));
        }
        if self.tables.len() > u16::MAX as usize {
            return Err(FontError::Generic("SFNT: too many tables".to_string()));
        }

        let num_tables = self.tables.len() as u16;
        let directory_end = 12 + self.tables.len() * TABLE_DIR_ENTRY_SIZE;

        let mut writer = Writer::new();
        writer.write_u32(self.flavor)?;
        writer.write_u16(num_tables)?;
        let (search_range, entry_selector, range_shift) = table_header_fields(num_tables);
        writer.write_u16(search_range)?;
        writer.write_u16(entry_selector)?;
        writer.write_u16(range_shift)?;

        // Reserve the directory; records are filled in after laying out data.
        writer.data.resize(directory_end, 0);

        let mut records = Vec::with_capacity(self.tables.len());
        let mut head_offset = None;

        for (tag, mut data) in std::mem::take(&mut self.tables) {
            if tag == Tag(*b"head") {
                if data.len() < HEAD_CHECKSUM_ADJUSTMENT_OFFSET + 4 {
                    return Err(FontError::Generic(
                        "SFNT: head table is too short".to_string(),
                    ));
                }
                // The directory checksum and the font checksum are both
                // computed with checkSumAdjustment set to zero.
                data[HEAD_CHECKSUM_ADJUSTMENT_OFFSET..HEAD_CHECKSUM_ADJUSTMENT_OFFSET + 4].fill(0);
                head_offset = Some(writer.data.len());
            }

            let offset = writer.data.len() as u32;
            let length = data.len() as u32;
            let checksum = calc_sfnt_checksum(&data);

            writer.data.extend_from_slice(&data);
            pad4(&mut writer.data);

            records.push(TableRecord {
                tag,
                checksum,
                offset,
                length,
            });
        }

        let mut directory = Writer::new();
        for record in &records {
            record.write_to(&mut directory)?;
        }
        writer.data[12..directory_end].copy_from_slice(&directory.data);

        let font_checksum = calc_sfnt_checksum(&writer.data);
        let checksum_adjustment = SFNT_CHECKSUM_MAGIC.wrapping_sub(font_checksum);

        let Some(head_offset) = head_offset else {
            return Err(FontError::Generic("SFNT: missing head table".to_string()));
        };
        writer.data[head_offset + HEAD_CHECKSUM_ADJUSTMENT_OFFSET
            ..head_offset + HEAD_CHECKSUM_ADJUSTMENT_OFFSET + 4]
            .copy_from_slice(&checksum_adjustment.to_be_bytes());

        Ok(writer.data)
    }
}

/// Serialize a full SFNT font from `(tag, data)` pairs.
pub fn assemble_sfnt<I>(flavor: u32, tables: I) -> Result<Vec<u8>, FontError>
where
    I: IntoIterator<Item = (Tag, Vec<u8>)>,
{
    let mut repack = SfntRepack::new(flavor);
    repack.extend(tables);
    repack.build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rfont_types::{ReadBytes, Reader, SFNT_VERSION_TTF};

    fn head_table() -> Vec<u8> {
        let mut head = vec![0u8; 54];
        head[8..12].copy_from_slice(&0xDEAD_BEEFu32.to_be_bytes());
        head
    }

    fn parse_directory(data: &[u8]) -> (u32, Vec<TableRecord>) {
        let mut reader = Reader::new(data);
        let flavor = reader.read_u32().unwrap();
        let num_tables = reader.read_u16().unwrap();
        reader.skip(6).unwrap(); // searchRange, entrySelector, rangeShift
        let mut records = Vec::new();
        for _ in 0..num_tables {
            records.push(TableRecord::read_from(&mut reader).unwrap());
        }
        (flavor, records)
    }

    #[test]
    fn test_table_header_fields() {
        assert_eq!(table_header_fields(0), (0, 0, 0));
        assert_eq!(table_header_fields(1), (16, 0, 0));
        assert_eq!(table_header_fields(10), (128, 3, 32));
        assert_eq!(table_header_fields(16), (256, 4, 0));
        assert_eq!(table_header_fields(17), (256, 4, 16));
    }

    #[test]
    fn test_repack_sorts_directory_and_patches_checksum() {
        let mut repack = SfntRepack::new(SFNT_VERSION_TTF);
        repack.insert(Tag(*b"maxp"), vec![1, 2, 3]);
        repack.insert(Tag(*b"head"), head_table());
        repack.insert(Tag(*b"cmap"), vec![4, 5, 6, 7, 8]);

        let font = repack.build().unwrap();

        let (flavor, records) = parse_directory(&font);
        assert_eq!(flavor, SFNT_VERSION_TTF);
        assert_eq!(records.len(), 3);

        let tags: Vec<&str> = records.iter().map(|r| r.tag.as_str()).collect();
        assert_eq!(tags, vec!["cmap", "head", "maxp"]);

        for record in &records {
            let start = record.offset as usize;
            let end = start + record.length as usize;
            assert!(end <= font.len());
            assert_eq!(record.offset % 4, 0);
            if record.tag != Tag(*b"head") {
                assert_eq!(
                    record.checksum,
                    calc_sfnt_checksum(&font[start..end]),
                    "checksum mismatch for {}",
                    record.tag
                );
            }
        }

        assert_eq!(calc_sfnt_checksum(&font), SFNT_CHECKSUM_MAGIC);
    }

    #[test]
    fn test_repack_head_directory_checksum_uses_zero_adjustment() {
        let mut head = head_table();
        let mut repack = SfntRepack::new(SFNT_VERSION_TTF);
        repack.insert(Tag(*b"head"), head.clone());
        let font = repack.build().unwrap();

        let (_, records) = parse_directory(&font);
        let record = records.iter().find(|r| r.tag == Tag(*b"head")).unwrap();

        head[8..12].fill(0);
        assert_eq!(record.checksum, calc_sfnt_checksum(&head));

        let start = record.offset as usize;
        assert_ne!(&font[start + 8..start + 12], &[0u8; 4]);
    }

    #[test]
    fn test_repack_preserves_otf_flavor() {
        let flavor = 0x4F54_544F; // 'OTTO'
        let font = assemble_sfnt(flavor, [(Tag(*b"head"), head_table())]).unwrap();
        assert_eq!(parse_directory(&font).0, flavor);
    }

    #[test]
    fn test_repack_pads_tables_to_four_bytes() {
        let font = assemble_sfnt(
            SFNT_VERSION_TTF,
            [
                (Tag(*b"head"), head_table()),
                (Tag(*b"maxp"), vec![0xAA; 5]),
            ],
        )
        .unwrap();

        assert_eq!(font.len() % 4, 0);
        let (_, records) = parse_directory(&font);
        let maxp = records.iter().find(|r| r.tag == Tag(*b"maxp")).unwrap();
        assert_eq!(maxp.length, 5);
        let start = maxp.offset as usize;
        assert_eq!(&font[start + 5..start + 8], &[0, 0, 0]);
    }

    #[test]
    fn test_repack_requires_head() {
        let result = assemble_sfnt(SFNT_VERSION_TTF, [(Tag(*b"maxp"), vec![0; 6])]);
        assert!(result.is_err());
    }

    #[test]
    fn test_repack_duplicate_tag_keeps_last() {
        let font = assemble_sfnt(
            SFNT_VERSION_TTF,
            [
                (Tag(*b"head"), head_table()),
                (Tag(*b"maxp"), vec![1, 1]),
                (Tag(*b"maxp"), vec![2, 2]),
            ],
        )
        .unwrap();

        let (_, records) = parse_directory(&font);
        let maxp = records.iter().find(|r| r.tag == Tag(*b"maxp")).unwrap();
        assert_eq!(maxp.length, 2);
        let start = maxp.offset as usize;
        assert_eq!(&font[start..start + 2], &[2, 2]);
    }
}
