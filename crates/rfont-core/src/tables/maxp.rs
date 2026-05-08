use rfont_types::{FontError, Reader, ReadBytes, Writer, WriteBytes};

#[derive(Debug, Clone)]
pub struct Maxp {
    pub version: u32,
    pub num_glyphs: u16,
    // v1.0 fields
    pub max_points: Option<u16>,
    pub max_contours: Option<u16>,
    pub max_composite_points: Option<u16>,
    pub max_composite_contours: Option<u16>,
    pub max_zones: Option<u16>,
    pub max_twilight_points: Option<u16>,
    pub max_storage: Option<u16>,
    pub max_function_defs: Option<u16>,
    pub max_instruction_defs: Option<u16>,
    pub max_stack_elements: Option<u16>,
    pub max_size_of_instructions: Option<u16>,
    pub max_component_elements: Option<u16>,
    pub max_component_depth: Option<u16>,
}

impl<'a> ReadBytes<'a> for Maxp {
    fn read_from(reader: &mut Reader<'a>) -> Result<Self, FontError> {
        let version = reader.read_u32()?;
        let num_glyphs = reader.read_u16()?;

        if version == 0x00005000 {
            // Version 0.5
            return Ok(Self {
                version,
                num_glyphs,
                max_points: None,
                max_contours: None,
                max_composite_points: None,
                max_composite_contours: None,
                max_zones: None,
                max_twilight_points: None,
                max_storage: None,
                max_function_defs: None,
                max_instruction_defs: None,
                max_stack_elements: None,
                max_size_of_instructions: None,
                max_component_elements: None,
                max_component_depth: None,
            });
        }

        // Version 1.0
        Ok(Self {
            version,
            num_glyphs,
            max_points: Some(reader.read_u16()?),
            max_contours: Some(reader.read_u16()?),
            max_composite_points: Some(reader.read_u16()?),
            max_composite_contours: Some(reader.read_u16()?),
            max_zones: Some(reader.read_u16()?),
            max_twilight_points: Some(reader.read_u16()?),
            max_storage: Some(reader.read_u16()?),
            max_function_defs: Some(reader.read_u16()?),
            max_instruction_defs: Some(reader.read_u16()?),
            max_stack_elements: Some(reader.read_u16()?),
            max_size_of_instructions: Some(reader.read_u16()?),
            max_component_elements: Some(reader.read_u16()?),
            max_component_depth: Some(reader.read_u16()?),
        })
    }
}

impl WriteBytes for Maxp {
    fn write_to(&self, writer: &mut Writer) -> Result<(), FontError> {
        writer.write_u32(self.version)?;
        writer.write_u16(self.num_glyphs)?;

        if self.version == 0x00005000 {
            return Ok(());
        }

        // Version 1.0 fields
        writer.write_u16(self.max_points.unwrap_or(0))?;
        writer.write_u16(self.max_contours.unwrap_or(0))?;
        writer.write_u16(self.max_composite_points.unwrap_or(0))?;
        writer.write_u16(self.max_composite_contours.unwrap_or(0))?;
        writer.write_u16(self.max_zones.unwrap_or(0))?;
        writer.write_u16(self.max_twilight_points.unwrap_or(0))?;
        writer.write_u16(self.max_storage.unwrap_or(0))?;
        writer.write_u16(self.max_function_defs.unwrap_or(0))?;
        writer.write_u16(self.max_instruction_defs.unwrap_or(0))?;
        writer.write_u16(self.max_stack_elements.unwrap_or(0))?;
        writer.write_u16(self.max_size_of_instructions.unwrap_or(0))?;
        writer.write_u16(self.max_component_elements.unwrap_or(0))?;
        writer.write_u16(self.max_component_depth.unwrap_or(0))?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rfont_types::io::Reader;

    #[test]
    fn test_maxp_version_05() {
        // Version 0.5 (only has num_glyphs)
        let data = vec![
            0x00, 0x00, 0x50, 0x00, // version = 0.5
            0x00, 0x10,             // num_glyphs = 16
        ];
        let mut reader = Reader::new(&data);
        let maxp = Maxp::read_from(&mut reader).unwrap();
        
        assert_eq!(maxp.version, 0x00005000);
        assert_eq!(maxp.num_glyphs, 16);
        assert!(maxp.max_points.is_none());
    }

    #[test]
    fn test_maxp_version_10() {
        // Version 1.0 (full fields)
        let mut data = vec![
            0x00, 0x01, 0x00, 0x00, // version = 1.0
            0x00, 0x20,             // num_glyphs = 32
        ];
        // 添加 v1.0 的字段（13个 u16）
        for i in 0..13 {
            data.push((i + 1) as u8);
            data.push(0);
        }
        
        let mut reader = Reader::new(&data);
        let maxp = Maxp::read_from(&mut reader).unwrap();
        
        assert_eq!(maxp.version, 0x00010000);
        assert_eq!(maxp.num_glyphs, 32);
        assert!(maxp.max_points.is_some());
    }

    #[test]
    fn test_maxp_write_version_05() {
        let maxp = Maxp {
            version: 0x00005000,
            num_glyphs: 16,
            max_points: None,
            max_contours: None,
            max_composite_points: None,
            max_composite_contours: None,
            max_zones: None,
            max_twilight_points: None,
            max_storage: None,
            max_function_defs: None,
            max_instruction_defs: None,
            max_stack_elements: None,
            max_size_of_instructions: None,
            max_component_elements: None,
            max_component_depth: None,
        };
        
        let mut writer = Writer::new();
        maxp.write_to(&mut writer).unwrap();
        
        // Version 0.5 should only write 6 bytes (4 + 2)
        assert_eq!(writer.data.len(), 6);
        assert_eq!(&writer.data[0..4], &[0x00, 0x00, 0x50, 0x00]);
        assert_eq!(&writer.data[4..6], &[0x00, 0x10]);
    }

    #[test]
    fn test_maxp_write_version_10() {
        let maxp = Maxp {
            version: 0x00010000,
            num_glyphs: 32,
            max_points: Some(100),
            max_contours: Some(10),
            max_composite_points: Some(50),
            max_composite_contours: Some(5),
            max_zones: Some(2),
            max_twilight_points: Some(20),
            max_storage: Some(10),
            max_function_defs: Some(5),
            max_instruction_defs: Some(3),
            max_stack_elements: Some(64),
            max_size_of_instructions: Some(128),
            max_component_elements: Some(4),
            max_component_depth: Some(2),
        };
        
        let mut writer = Writer::new();
        maxp.write_to(&mut writer).unwrap();
        
        // Version 1.0 should write 32 bytes (4 + 2 + 13*2)
        assert_eq!(writer.data.len(), 32);
    }

    #[test]
    fn test_maxp_roundtrip_version_05() {
        let original = Maxp {
            version: 0x00005000,
            num_glyphs: 20,
            max_points: None,
            max_contours: None,
            max_composite_points: None,
            max_composite_contours: None,
            max_zones: None,
            max_twilight_points: None,
            max_storage: None,
            max_function_defs: None,
            max_instruction_defs: None,
            max_stack_elements: None,
            max_size_of_instructions: None,
            max_component_elements: None,
            max_component_depth: None,
        };
        
        let mut writer = Writer::new();
        original.write_to(&mut writer).unwrap();
        
        let mut reader = Reader::new(&writer.data);
        let restored = Maxp::read_from(&mut reader).unwrap();
        
        assert_eq!(restored.version, original.version);
        assert_eq!(restored.num_glyphs, original.num_glyphs);
    }

    #[test]
    fn test_maxp_large_num_glyphs() {
        // Test with maximum number of glyphs (u16::MAX)
        let mut data = vec![
            0x00, 0x01, 0x00, 0x00, // version = 1.0
            0xFF, 0xFF,             // num_glyphs = 65535
        ];
        // Add dummy v1.0 fields
        for _ in 0..13 {
            data.push(0);
            data.push(0);
        }
        
        let mut reader = Reader::new(&data);
        let maxp = Maxp::read_from(&mut reader).unwrap();
        
        assert_eq!(maxp.num_glyphs, 65535);
    }
}
