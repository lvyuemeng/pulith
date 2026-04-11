use std::io::{self, Read, Seek};

use crate::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArchiveFormat {
    Zip,
    Tar(TarCompress),
}

/// Compression codec for tar archives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TarCompress {
    None,
    Gzip,
    Xz,
    Zstd,
}

impl TarCompress {
    /// Create a decoder for this compression codec.
    pub fn decoder<R: Read + 'static>(self, reader: R) -> Result<Decoder, Error> {
        match self {
            Self::None => Ok(Decoder::new(reader)),
            Self::Gzip => Ok(Decoder::new(flate2::read::GzDecoder::new(reader))),
            #[cfg(feature = "xz")]
            Self::Xz => Ok(Decoder::new(xz2::read::XzDecoder::new(reader))),
            #[cfg(not(feature = "xz"))]
            Self::Xz => Err(Error::UnsupportedFormat),
            #[cfg(feature = "zstd")]
            Self::Zstd => {
                let decoder = zstd::stream::Decoder::new(reader).map_err(|_| Error::Corrupted)?;
                Ok(Decoder::new(decoder))
            }
            #[cfg(not(feature = "zstd"))]
            Self::Zstd => Err(Error::UnsupportedFormat),
        }
    }
}

/// Decoder wrapper for tar decompression.
pub struct Decoder {
    inner: Box<dyn Read>,
}

impl Decoder {
    fn new<R: Read + 'static>(reader: R) -> Self {
        Self {
            inner: Box::new(reader),
        }
    }
}

impl std::fmt::Debug for Decoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Decoder(..)")
    }
}

impl Read for Decoder {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(buf)
    }
}

/// Detect archive format from file extension.
///
/// Recognized extensions:
/// - `.zip`
/// - `.tar`, `.tar.gz`, `.tgz`
/// - `.tar.xz`, `.txz`
/// - `.tar.zst`, `.tzst`
pub fn detect_from_filename(path: &std::path::Path) -> Option<ArchiveFormat> {
    let name = path.file_name()?.to_string_lossy().to_ascii_lowercase();

    if name.ends_with(".zip") {
        return Some(ArchiveFormat::Zip);
    }
    if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        return Some(ArchiveFormat::Tar(TarCompress::Gzip));
    }
    if name.ends_with(".tar.xz") || name.ends_with(".txz") {
        return Some(ArchiveFormat::Tar(TarCompress::Xz));
    }
    if name.ends_with(".tar.zst") || name.ends_with(".tzst") {
        return Some(ArchiveFormat::Tar(TarCompress::Zstd));
    }
    if name.ends_with(".tar") {
        return Some(ArchiveFormat::Tar(TarCompress::None));
    }

    None
}
pub fn detect_format(data: &[u8]) -> Option<ArchiveFormat> {
    match data {
        [0x50, 0x4B, 0x03, 0x04, ..] => Some(ArchiveFormat::Zip),
        [0x1F, 0x8B, ..] => Some(ArchiveFormat::Tar(TarCompress::Gzip)),
        [0x28, 0xB5, 0x2F, 0xFD, ..] => Some(ArchiveFormat::Tar(TarCompress::Zstd)),
        [0xFD, 0x37, 0x7A, 0x58, 0x5A, 0x00, ..] => Some(ArchiveFormat::Tar(TarCompress::Xz)),
        _ => {
            if is_tar_header(data) {
                Some(ArchiveFormat::Tar(TarCompress::None))
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
mod tests_detect {
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
            Some(ArchiveFormat::Tar(TarCompress::Gzip))
        );
    }

    #[test]
    fn detect_tar_zstd_format() {
        let zstd_header = [0x28, 0xB5, 0x2F, 0xFD, 0x00, 0x00, 0x00, 0x00];
        assert_eq!(
            detect_format(&zstd_header),
            Some(ArchiveFormat::Tar(TarCompress::Zstd))
        );
    }

    #[test]
    fn detect_tar_xz_format() {
        let xz_header = [0xFD, 0x37, 0x7A, 0x58, 0x5A, 0x00, 0x00, 0x00];
        assert_eq!(
            detect_format(&xz_header),
            Some(ArchiveFormat::Tar(TarCompress::Xz))
        );
    }

    #[test]
    fn detect_tar_plain_format() {
        let mut tar_header = [0u8; 512];
        tar_header[257..263].copy_from_slice(b"ustar\0");
        assert_eq!(
            detect_format(&tar_header),
            Some(ArchiveFormat::Tar(TarCompress::None))
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
        assert_eq!(format, Some(ArchiveFormat::Tar(TarCompress::Gzip)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn archive_format_variants() {
        assert_ne!(ArchiveFormat::Zip, ArchiveFormat::Tar(TarCompress::None));
        assert_ne!(ArchiveFormat::Zip, ArchiveFormat::Tar(TarCompress::Gzip));
    }

    #[test]
    fn archive_format_tar_with_compression() {
        assert_eq!(
            ArchiveFormat::Tar(TarCompress::None),
            ArchiveFormat::Tar(TarCompress::None)
        );
        assert_eq!(
            ArchiveFormat::Tar(TarCompress::Gzip),
            ArchiveFormat::Tar(TarCompress::Gzip)
        );
        assert_eq!(
            ArchiveFormat::Tar(TarCompress::Xz),
            ArchiveFormat::Tar(TarCompress::Xz)
        );
        assert_eq!(
            ArchiveFormat::Tar(TarCompress::Zstd),
            ArchiveFormat::Tar(TarCompress::Zstd)
        );
    }

    #[test]
    fn compression_variants() {
        let variants = [
            TarCompress::None,
            TarCompress::Gzip,
            TarCompress::Xz,
            TarCompress::Zstd,
        ];
        for (i, compression) in variants.iter().enumerate() {
            match i {
                0 => assert!(matches!(compression, TarCompress::None)),
                1 => assert!(matches!(compression, TarCompress::Gzip)),
                2 => assert!(matches!(compression, TarCompress::Xz)),
                3 => assert!(matches!(compression, TarCompress::Zstd)),
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn archive_format_equality() {
        assert_eq!(ArchiveFormat::Zip, ArchiveFormat::Zip);
        assert_eq!(
            ArchiveFormat::Tar(TarCompress::Gzip),
            ArchiveFormat::Tar(TarCompress::Gzip)
        );
        assert_ne!(
            ArchiveFormat::Tar(TarCompress::Gzip),
            ArchiveFormat::Tar(TarCompress::None)
        );
        assert_ne!(ArchiveFormat::Zip, ArchiveFormat::Tar(TarCompress::None));
    }

    #[test]
    fn archive_format_clone() {
        let format = ArchiveFormat::Tar(TarCompress::Zstd);
        let cloned = format;
        assert_eq!(format, cloned);
    }

    #[test]
    fn compression_clone() {
        let compression = TarCompress::Xz;
        let cloned = compression;
        assert_eq!(compression, cloned);
    }

    #[test]
    fn compression_none_decoder() {
        let data = b"hello";
        let mut decoder = TarCompress::None.decoder(Cursor::new(data)).unwrap();
        let mut output = String::new();
        decoder.read_to_string(&mut output).unwrap();
        assert_eq!(output, "hello");
    }

    #[test]
    fn detect_from_filename_variants() {
        assert_eq!(
            detect_from_filename(std::path::Path::new("runtime.zip")),
            Some(ArchiveFormat::Zip)
        );
        assert_eq!(
            detect_from_filename(std::path::Path::new("runtime.tar.gz")),
            Some(ArchiveFormat::Tar(TarCompress::Gzip))
        );
        assert_eq!(
            detect_from_filename(std::path::Path::new("runtime.tar.xz")),
            Some(ArchiveFormat::Tar(TarCompress::Xz))
        );
        assert_eq!(
            detect_from_filename(std::path::Path::new("runtime.tar.zst")),
            Some(ArchiveFormat::Tar(TarCompress::Zstd))
        );
        assert_eq!(
            detect_from_filename(std::path::Path::new("runtime.tar")),
            Some(ArchiveFormat::Tar(TarCompress::None))
        );
        assert_eq!(
            detect_from_filename(std::path::Path::new("runtime.unknown")),
            None
        );
    }

    #[test]
    fn compression_gzip_decoder() {
        let data = vec![0x1f, 0x8b];
        let mut decoder = TarCompress::Gzip.decoder(Cursor::new(data)).unwrap();
        let mut output = [0u8; 1];
        assert!(decoder.read(&mut output).is_err());
    }

    #[test]
    #[cfg(not(feature = "xz"))]
    fn compression_xz_unsupported() {
        let data = Vec::new();
        let result = TarCompress::Xz.decoder(Cursor::new(data));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::UnsupportedFormat));
    }
}
