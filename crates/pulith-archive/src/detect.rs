use std::io::{self, Read, Seek};

use crate::data::archive::{ArchiveFormat, Compression};

pub fn detect_format(data: &[u8]) -> Option<ArchiveFormat> {
    match data {
        [0x50, 0x4B, 0x03, 0x04, ..] => Some(ArchiveFormat::Zip),
        [0x1F, 0x8B, ..] => Some(ArchiveFormat::Tar(Compression::Gzip)),
        [0x28, 0xB5, 0x2F, 0xFD, ..] => Some(ArchiveFormat::Tar(Compression::Zstd)),
        [0xFD, 0x37, 0x7A, 0x58, 0x5A, 0x00, ..] => Some(ArchiveFormat::Tar(Compression::Xz)),
        _ => {
            if is_tar_header(data) {
                Some(ArchiveFormat::Tar(Compression::None))
            } else {
                None
            }
        }
    }
}

fn is_tar_header(data: &[u8]) -> bool {
    data.len() >= 512 && data[257..263] == *b"ustar\0"
}

pub fn detect_from_reader<R: Read + Seek>(reader: &mut R) -> io::Result<Option<ArchiveFormat>> {
    let mut header = [0u8; 32];
    reader.read_exact(&mut header)?;
    reader.rewind()?;
    Ok(detect_format(&header))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_zip_format() {
        let zip_header = [0x50, 0x4B, 0x03, 0x04, 0x14, 0x00, 0x00, 0x00];
        assert_eq!(detect_format(&zip_header), Some(ArchiveFormat::Zip));
    }

    #[test]
    fn detect_tar_gz_format() {
        let gz_header = [0x1F, 0x8B, 0x08, 0x00];
        assert_eq!(
            detect_format(&gz_header),
            Some(ArchiveFormat::Tar(Compression::Gzip))
        );
    }

    #[test]
    fn detect_tar_zstd_format() {
        let zstd_header = [0x28, 0xB5, 0x2F, 0xFD, 0x00, 0x00, 0x00, 0x00];
        assert_eq!(
            detect_format(&zstd_header),
            Some(ArchiveFormat::Tar(Compression::Zstd))
        );
    }

    #[test]
    fn detect_tar_xz_format() {
        let xz_header = [0xFD, 0x37, 0x7A, 0x58, 0x5A, 0x00, 0x00, 0x00];
        assert_eq!(
            detect_format(&xz_header),
            Some(ArchiveFormat::Tar(Compression::Xz))
        );
    }

    #[test]
    fn detect_tar_plain_format() {
        let mut tar_header = [0u8; 512];
        tar_header[257..263].copy_from_slice(b"ustar\0");
        assert_eq!(
            detect_format(&tar_header),
            Some(ArchiveFormat::Tar(Compression::None))
        );
    }

    #[test]
    fn detect_unknown_format() {
        let random_data = [0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x00, 0x00, 0x00];
        assert_eq!(detect_format(&random_data), None);
    }

    #[test]
    fn detect_truncated_tar_header() {
        let short_data = [0u8; 256];
        assert_eq!(detect_format(&short_data), None);
    }

    #[test]
    fn detect_zip_from_file() {
        let data = vec![
            0x50, 0x4B, 0x03, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ];
        let mut cursor = io::Cursor::new(data);
        let format = detect_from_reader(&mut cursor).unwrap();
        assert_eq!(format, Some(ArchiveFormat::Zip));
    }

    #[test]
    fn detect_tar_gz_from_file() {
        let data = vec![
            0x1F, 0x8B, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ];
        let mut cursor = io::Cursor::new(data);
        let format = detect_from_reader(&mut cursor).unwrap();
        assert_eq!(format, Some(ArchiveFormat::Tar(Compression::Gzip)));
    }
}
