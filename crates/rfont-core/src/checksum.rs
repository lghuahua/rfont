/// 计算 SFNT 校验和（32 位累加，数据按 4 字节对齐）
///
/// SFNT 校验和算法是 OpenType/TrueType 字体规范中使用的校验和计算方法。
/// 它将数据按 4 字节分组，累加所有 32 位值，最后取低 32 位。
///
/// # 参数
/// - `data`: 要计算校验和的字节数据
///
/// # 返回值
/// 32 位校验和值
///
/// # 注意
/// - 如果数据长度不是 4 的倍数，会在末尾填充零字节
/// - 此函数用于计算字体表的校验和，以及整个字体的 checkSumAdjustment
///
/// # 示例
/// ```
/// use rfont_core::calc_sfnt_checksum;
///
/// let data = vec![0x00, 0x01, 0x00, 0x00];
/// let checksum = calc_sfnt_checksum(&data);
/// assert_eq!(checksum, 0x00010000);
/// ```
pub fn calc_sfnt_checksum(data: &[u8]) -> u32 {
    let mut sum: u64 = 0;
    let mut i = 0;

    // 每次处理 4 字节
    while i + 4 <= data.len() {
        let val = u32::from_be_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]);
        sum += val as u64;
        i += 4;
    }

    // 处理剩余的字节（填充到 4 字节）
    if i < data.len() {
        let mut bytes = [0u8; 4];
        let remaining = &data[i..];
        bytes[..remaining.len()].copy_from_slice(remaining);
        sum += u32::from_be_bytes(bytes) as u64;
    }

    (sum & 0xFFFFFFFF) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sfnt_checksum_calculation() {
        let data = vec![0x00, 0x01, 0x00, 0x00];
        let checksum = calc_sfnt_checksum(&data);
        assert_eq!(checksum, 0x00010000);

        let data2 = vec![0x00, 0x01, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00];
        let checksum2 = calc_sfnt_checksum(&data2);
        assert_eq!(checksum2, 0x00030000);
    }

    #[test]
    fn test_sfnt_checksum_padding() {
        let data = vec![0x01, 0x02, 0x03];
        let checksum = calc_sfnt_checksum(&data);
        assert_eq!(checksum, 0x01020300);

        let data2 = vec![0xFF];
        let checksum2 = calc_sfnt_checksum(&data2);
        assert_eq!(checksum2, 0xFF000000);
    }
}
