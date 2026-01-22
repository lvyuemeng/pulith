#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArchiveFormat {
    Zip,
    Tar(Compression),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Compression {
    None,
    Gzip,
    Xz,
    Zstd,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archive_format_variants() {
        assert_ne!(ArchiveFormat::Zip, ArchiveFormat::Tar(Compression::None));
        assert_ne!(ArchiveFormat::Zip, ArchiveFormat::Tar(Compression::Gzip));
    }

    #[test]
    fn archive_format_tar_with_compression() {
        assert_eq!(
            ArchiveFormat::Tar(Compression::None),
            ArchiveFormat::Tar(Compression::None)
        );
        assert_eq!(
            ArchiveFormat::Tar(Compression::Gzip),
            ArchiveFormat::Tar(Compression::Gzip)
        );
        assert_eq!(
            ArchiveFormat::Tar(Compression::Xz),
            ArchiveFormat::Tar(Compression::Xz)
        );
        assert_eq!(
            ArchiveFormat::Tar(Compression::Zstd),
            ArchiveFormat::Tar(Compression::Zstd)
        );
    }

    #[test]
    fn compression_variants() {
        let variants = [
            Compression::None,
            Compression::Gzip,
            Compression::Xz,
            Compression::Zstd,
        ];
        for (i, compression) in variants.iter().enumerate() {
            match i {
                0 => assert!(matches!(compression, Compression::None)),
                1 => assert!(matches!(compression, Compression::Gzip)),
                2 => assert!(matches!(compression, Compression::Xz)),
                3 => assert!(matches!(compression, Compression::Zstd)),
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn archive_format_equality() {
        assert_eq!(ArchiveFormat::Zip, ArchiveFormat::Zip);
        assert_eq!(
            ArchiveFormat::Tar(Compression::Gzip),
            ArchiveFormat::Tar(Compression::Gzip)
        );
        assert_ne!(
            ArchiveFormat::Tar(Compression::Gzip),
            ArchiveFormat::Tar(Compression::None)
        );
        assert_ne!(ArchiveFormat::Zip, ArchiveFormat::Tar(Compression::None));
    }

    #[test]
    fn archive_format_clone() {
        let format = ArchiveFormat::Tar(Compression::Zstd);
        let cloned = format;
        assert_eq!(format, cloned);
    }

    #[test]
    fn compression_clone() {
        let compression = Compression::Xz;
        let cloned = compression;
        assert_eq!(compression, cloned);
    }
}
