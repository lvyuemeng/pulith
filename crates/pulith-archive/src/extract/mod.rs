use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};

use pulith_fs::Workspace;

use crate::data::archive::{ArchiveFormat, Compression};
use crate::data::options::{ExtractionOptions, HashStrategy, PermissionStrategy, Progress};
use crate::data::report::{ArchiveReport, ExtractedEntry};
use crate::error::{Error, Result};
use crate::sanitize::{sanitize_path, sanitize_symlink_target, strip_path_components};

mod tar_codecs;

pub enum ArchiveExtractor {
    Zip(ZipExtractor),
    Tar(TarExtractor),
}

pub struct ZipExtractor;

pub struct TarExtractor {
    codec: Compression,
}

impl TarExtractor {
    pub fn new(codec: Compression) -> Self {
        Self { codec }
    }
}

fn apply_permission_strategy(
    _path: &Path,
    _permissions: Option<u32>,
    _strategy: PermissionStrategy,
) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        match strategy {
            PermissionStrategy::Standard => {
                if let Some(mode) = permissions {
                    let perms = if mode & 0o111 != 0 {
                        PermissionsExt::from_mode(mode)
                    } else {
                        PermissionsExt::from_mode(mode | 0o644)
                    };
                    std::fs::set_permissions(path, perms)?;
                } else {
                    let perms = PermissionsExt::from_mode(0o644);
                    std::fs::set_permissions(path, perms)?;
                }
            }
            PermissionStrategy::ReadOnly => {
                let perms = PermissionsExt::from_mode(0o444);
                std::fs::set_permissions(path, perms)?;
            }
            PermissionStrategy::Preserve => {
                if let Some(mode) = permissions {
                    let perms = PermissionsExt::from_mode(mode);
                    std::fs::set_permissions(path, perms)?;
                }
            }
            PermissionStrategy::Owned => {
                let perms = PermissionsExt::from_mode(0o644);
                std::fs::set_permissions(path, perms)?;
            }
        }
    }
    Ok(())
}

fn calculate_hash(content: &[u8], strategy: HashStrategy) -> Option<String> {
    match strategy {
        HashStrategy::None => None,
        HashStrategy::Sha256 => {
            use sha2::Digest;
            let mut hasher = sha2::Sha256::new();
            hasher.update(content);
            Some(format!("{:x}", hasher.finalize()))
        }
        HashStrategy::Blake3 => {
            let mut hasher = blake3::Hasher::new();
            hasher.update(content);
            Some(format!("{}", hasher.finalize()))
        }
    }
}

impl ArchiveExtractor {
    pub fn extract<R: Read + Seek + 'static>(
        &self,
        reader: R,
        destination: &Path,
        options: &ExtractionOptions,
        workspace: Option<&Workspace>,
    ) -> Result<ArchiveReport> {
        match self {
            ArchiveExtractor::Zip(extractor) => {
                extractor.extract(reader, destination, options, workspace)
            }
            ArchiveExtractor::Tar(extractor) => {
                extractor.extract(reader, destination, options, workspace)
            }
        }
    }
}

impl ZipExtractor {
    pub fn extract<R: Read + Seek>(
        &self,
        mut reader: R,
        destination: &Path,
        options: &ExtractionOptions,
        workspace: Option<&Workspace>,
    ) -> Result<ArchiveReport> {
        let mut archive = zip::ZipArchive::new(&mut reader).map_err(|_| Error::Corrupted)?;

        let mut entries = Vec::new();
        let mut total_bytes = 0u64;
        let mut bytes_processed = 0u64;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i).map_err(|_| Error::Corrupted)?;

            let raw_path = file
                .enclosed_name()
                .ok_or(Error::InvalidPath)?
                .to_path_buf();

            let sanitized =
                if options.strip_components > 0 {
                    let stripped = strip_path_components(&raw_path, options.strip_components)
                        .map_err(|_| Error::NoComponentsRemaining {
                            original: raw_path.clone(),
                            count: options.strip_components,
                        })?;
                    sanitize_path(&stripped, destination)?
                } else {
                    sanitize_path(&raw_path, destination)?
                };

            let size = file.size();
            let is_dir = file.is_dir();

            let symlink_indicator = raw_path.as_os_str().to_string_lossy();
            let is_symlink =
                symlink_indicator.ends_with(".lnk") || symlink_indicator.contains(".lnk");

            let extraction_target = workspace
                .map(|w| w.path().join(&sanitized.resolved))
                .unwrap_or_else(|| sanitized.resolved.clone());

            let mut content = Vec::new();
            file.read_to_end(&mut content)?;

            let hash = if options.hash_strategy != HashStrategy::None {
                calculate_hash(&content, options.hash_strategy)
            } else {
                None
            };

            if !is_dir && !is_symlink {
                if let Some(parent) = extraction_target.parent() {
                    if !parent.exists() {
                        std::fs::create_dir_all(parent).map_err(|e| {
                            Error::DirectoryCreationFailed {
                                path: parent.to_path_buf(),
                                source: e,
                            }
                        })?;
                    }
                }

                let mut out_file = std::fs::File::create(&extraction_target).map_err(|e| {
                    Error::ExtractionFailed {
                        path: extraction_target.clone(),
                        source: e,
                    }
                })?;
                out_file.write_all(&content)?;

                #[cfg(unix)]
                {
                    let permissions = file.external_attributes().map(|a| (a >> 16) as u32);
                    apply_permission_strategy(
                        &extraction_target,
                        permissions,
                        options.permission_strategy,
                    )?;
                }
                #[cfg(not(unix))]
                {
                    apply_permission_strategy(
                        &extraction_target,
                        None,
                        options.permission_strategy,
                    )?;
                }
            } else if is_dir {
                if !extraction_target.exists() {
                    std::fs::create_dir_all(&extraction_target).map_err(|e| {
                        Error::DirectoryCreationFailed {
                            path: extraction_target.clone(),
                            source: e,
                        }
                    })?;
                }
            }

            bytes_processed += size;
            total_bytes += size;

            let original_path = sanitized.original.clone();

            if let Some(ref callback) = options.on_progress {
                callback(Progress {
                    bytes_processed,
                    total_bytes: if total_bytes > 0 {
                        Some(total_bytes)
                    } else {
                        None
                    },
                    percentage: None,
                    current_file: Some(original_path.clone()),
                });
            }

            let extracted_entry = ExtractedEntry {
                original_path: sanitized.original,
                target_path: sanitized.resolved,
                size,
                permissions: None,
                is_directory: is_dir,
                is_symlink,
                symlink_target: None,
                hash,
            };

            entries.push(extracted_entry);
        }

        Ok(ArchiveReport {
            format: ArchiveFormat::Zip,
            entry_count: entries.len(),
            total_bytes,
            entries,
        })
    }
}

impl TarExtractor {
    pub fn extract<R: Read + Seek + 'static>(
        &self,
        reader: R,
        destination: &Path,
        options: &ExtractionOptions,
        workspace: Option<&Workspace>,
    ) -> Result<ArchiveReport> {
        let decompressed = tar_codecs::wrap_reader(reader, self.codec)?;
        let mut archive = tar::Archive::new(decompressed);

        let mut entries = Vec::new();
        let mut total_bytes = 0u64;
        let mut bytes_processed = 0u64;

        for entry in archive.entries()? {
            let mut entry = entry.map_err(|_| Error::Corrupted)?;

            let raw_path = entry.path()?.into_owned();

            let sanitized =
                if options.strip_components > 0 {
                    let stripped = strip_path_components(&raw_path, options.strip_components)
                        .map_err(|_| Error::NoComponentsRemaining {
                            original: raw_path.clone(),
                            count: options.strip_components,
                        })?;
                    sanitize_path(&stripped, destination)?
                } else {
                    sanitize_path(&raw_path, destination)?
                };

            let header = entry.header();
            let size = header.size().unwrap_or(0);
            let entry_type = header.entry_type();
            let is_dir = entry_type.is_dir();
            let is_symlink = entry_type.is_symlink();
            let is_file = entry_type.is_file();

            let extraction_target = workspace
                .map(|w| w.path().join(&sanitized.resolved))
                .unwrap_or_else(|| sanitized.resolved.clone());

            let mut hash: Option<String> = None;

            if is_file {
                if let Some(parent) = extraction_target.parent() {
                    if !parent.exists() {
                        std::fs::create_dir_all(parent).map_err(|e| {
                            Error::DirectoryCreationFailed {
                                path: parent.to_path_buf(),
                                source: e,
                            }
                        })?;
                    }
                }

                let mut content = Vec::new();
                entry.read_to_end(&mut content)?;

                if options.hash_strategy != HashStrategy::None {
                    use sha2::Digest;
                    hash = Some(match options.hash_strategy {
                        HashStrategy::Sha256 => {
                            let mut hasher = sha2::Sha256::new();
                            hasher.update(&content);
                            format!("{:x}", hasher.finalize())
                        }
                        HashStrategy::Blake3 => {
                            let mut hasher = blake3::Hasher::new();
                            hasher.update(&content);
                            format!("{}", hasher.finalize())
                        }
                        _ => unreachable!(),
                    });
                }

                let mut out_file = std::fs::File::create(&extraction_target).map_err(|e| {
                    Error::ExtractionFailed {
                        path: extraction_target.clone(),
                        source: e,
                    }
                })?;
                out_file.write_all(&content)?;

                #[cfg(unix)]
                {
                    let permissions = header.mode().ok();
                    apply_permission_strategy(
                        &extraction_target,
                        permissions,
                        options.permission_strategy,
                    )?;
                }
                #[cfg(not(unix))]
                {
                    apply_permission_strategy(
                        &extraction_target,
                        None,
                        options.permission_strategy,
                    )?;
                }
            } else if is_dir {
                if !extraction_target.exists() {
                    std::fs::create_dir_all(&extraction_target).map_err(|e| {
                        Error::DirectoryCreationFailed {
                            path: extraction_target.clone(),
                            source: e,
                        }
                    })?;
                }

                #[cfg(unix)]
                {
                    let permissions = header.mode().ok();
                    apply_permission_strategy(
                        &extraction_target,
                        permissions,
                        options.permission_strategy,
                    )?;
                }
            } else if is_symlink {
                let target = entry.link_name()?.map(|p| p.into_owned()).ok_or(
                    Error::SymlinkCreationFailed {
                        target: PathBuf::new(),
                        link: extraction_target.clone(),
                        source: std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "missing symlink target",
                        ),
                    },
                )?;

                let sanitized_target =
                    sanitize_symlink_target(&target, &sanitized.resolved, destination)?;

                #[cfg(unix)]
                std::os::unix::fs::symlink(&sanitized_target, &extraction_target).map_err(|e| {
                    Error::SymlinkCreationFailed {
                        target: sanitized_target,
                        link: extraction_target.clone(),
                        source: e,
                    }
                })?;

                #[cfg(windows)]
                {
                    use std::os::windows::fs::symlink_dir;
                    let is_dir_target =
                        sanitized_target.is_dir() || target.to_string_lossy().ends_with('/');
                    if is_dir_target {
                        symlink_dir(&sanitized_target, &extraction_target).map_err(|e| {
                            Error::SymlinkCreationFailed {
                                target: sanitized_target,
                                link: extraction_target.clone(),
                                source: e,
                            }
                        })?;
                    } else {
                        std::os::windows::fs::symlink_file(&sanitized_target, &extraction_target)
                            .map_err(|e| Error::SymlinkCreationFailed {
                            target: sanitized_target,
                            link: extraction_target.clone(),
                            source: e,
                        })?;
                    }
                }

                hash = None;
            }

            bytes_processed += size;
            total_bytes += size;

            let original_path = sanitized.original.clone();

            if let Some(ref callback) = options.on_progress {
                callback(Progress {
                    bytes_processed,
                    total_bytes: None,
                    percentage: None,
                    current_file: Some(original_path.clone()),
                });
            }

            let symlink_target = if is_symlink {
                Some(sanitize_symlink_target(
                    &entry.link_name()?.unwrap_or_default(),
                    &sanitized.resolved,
                    destination,
                )?)
            } else {
                None
            };

            let permissions = None;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                permissions = header.mode().ok().map(|m| m);
            }

            let extracted_entry = ExtractedEntry {
                original_path,
                target_path: sanitized.resolved,
                size,
                permissions,
                is_directory: is_dir,
                is_symlink,
                symlink_target,
                hash,
            };

            entries.push(extracted_entry);
        }

        Ok(ArchiveReport {
            format: ArchiveFormat::Tar(self.codec),
            entry_count: entries.len(),
            total_bytes,
            entries,
        })
    }
}

pub fn extractor_for(format: ArchiveFormat) -> Option<ArchiveExtractor> {
    match format {
        ArchiveFormat::Zip => Some(ArchiveExtractor::Zip(ZipExtractor)),
        ArchiveFormat::Tar(codec) => Some(ArchiveExtractor::Tar(TarExtractor::new(codec))),
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use super::*;
    use crate::data::options::Progress;

    #[test]
    fn extractor_for_zip() {
        let extractor = extractor_for(ArchiveFormat::Zip);
        assert!(extractor.is_some());
        assert!(matches!(extractor.unwrap(), ArchiveExtractor::Zip(_)));
    }

    #[test]
    fn extractor_for_tar_gzip() {
        let extractor = extractor_for(ArchiveFormat::Tar(Compression::Gzip));
        assert!(extractor.is_some());
        if let ArchiveExtractor::Tar(tar) = extractor.unwrap() {
            assert_eq!(tar.codec, Compression::Gzip);
        }
    }

    #[test]
    fn extractor_for_tar_plain() {
        let extractor = extractor_for(ArchiveFormat::Tar(Compression::None));
        assert!(extractor.is_some());
        if let ArchiveExtractor::Tar(tar) = extractor.unwrap() {
            assert_eq!(tar.codec, Compression::None);
        }
    }

    #[test]
    fn extractor_for_tar_xz() {
        let extractor = extractor_for(ArchiveFormat::Tar(Compression::Xz));
        assert!(extractor.is_some());
    }

    #[test]
    fn extractor_for_tar_zstd() {
        let extractor = extractor_for(ArchiveFormat::Tar(Compression::Zstd));
        assert!(extractor.is_some());
    }

    #[test]
    fn archive_extractor_dispatch_zip() {
        let extractor = ArchiveExtractor::Zip(ZipExtractor);
        let data = Vec::new();
        let cursor = Cursor::new(data);
        let options = ExtractionOptions::default();
        let result = extractor.extract(cursor, Path::new("/tmp"), &options, None);
        assert!(result.is_err());
    }

    #[test]
    fn archive_extractor_dispatch_tar() {
        let extractor = ArchiveExtractor::Tar(TarExtractor::new(Compression::None));
        let data = Vec::new();
        let cursor = Cursor::new(data);
        let options = ExtractionOptions::default();
        let result = extractor.extract(cursor, Path::new("/tmp"), &options, None);
        assert!(result.is_ok());
    }

    #[test]
    fn extraction_options_progress_callback() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);

        let options = ExtractionOptions::default().on_progress(Arc::new(move |_: Progress| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        }));

        let progress = Progress {
            bytes_processed: 100,
            total_bytes: Some(1000),
            percentage: Some(10.0),
            current_file: Some(PathBuf::from("test.txt")),
        };

        if let Some(ref cb) = options.on_progress {
            cb(progress);
        }

        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn tar_extractor_new_with_compression() {
        let gzip_extractor = TarExtractor::new(Compression::Gzip);
        assert_eq!(gzip_extractor.codec, Compression::Gzip);

        let xz_extractor = TarExtractor::new(Compression::Xz);
        assert_eq!(xz_extractor.codec, Compression::Xz);

        let zstd_extractor = TarExtractor::new(Compression::Zstd);
        assert_eq!(zstd_extractor.codec, Compression::Zstd);

        let none_extractor = TarExtractor::new(Compression::None);
        assert_eq!(none_extractor.codec, Compression::None);
    }

    #[test]
    fn extract_actual_tar_gz_file() {
        use std::io::Cursor;
        
        const NAME:&str = "hello.txt";
        const CONTENT: &str = "Hello, World!";

        let tar_gz_data = include_bytes!("../../tests/fixtures/test.tar.gz");
        let cursor = Cursor::new(tar_gz_data.to_vec());

        let temp_dir = tempfile::tempdir().unwrap();
        let dest = temp_dir.path().join("output");

        let options =
            ExtractionOptions::default().permission_strategy(PermissionStrategy::Standard);

        let extractor = ArchiveExtractor::Tar(TarExtractor::new(Compression::Gzip));
        let result = extractor.extract(cursor, &dest, &options, None);

        assert!(
            result.is_ok(),
            "Extraction should succeed: {:?}",
            result.err()
        );
        let report = result.unwrap();
        assert!(report.entry_count > 0, "Should have extracted entries");
        assert!(report.total_bytes > 0, "Should have extracted bytes");

        let extracted_file = dest.join(NAME);
        assert!(
            extracted_file.exists(),
            "Extracted file should exist: {:?}",
            extracted_file
        );
        let content = std::fs::read_to_string(&extracted_file).unwrap();
        assert!(content.contains(CONTENT), "Content should match");
    }

    #[test]
    fn extract_tar_with_progress_callback() {
        use std::io::Cursor;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let tar_gz_data = include_bytes!("../../tests/fixtures/test.tar.gz");
        let cursor = Cursor::new(tar_gz_data.to_vec());

        let call_count = Arc::new(AtomicUsize::new(0));
        let call_count_clone = Arc::clone(&call_count);

        let temp_dir = tempfile::tempdir().unwrap();
        let dest = temp_dir.path().join("output");

        let options = ExtractionOptions::default().on_progress(Arc::new(move |_: Progress| {
            call_count_clone.fetch_add(1, Ordering::SeqCst);
        }));

        let extractor = ArchiveExtractor::Tar(TarExtractor::new(Compression::Gzip));
        let result = extractor.extract(cursor, &dest, &options, None);

        assert!(result.is_ok());
        assert!(
            call_count.load(Ordering::SeqCst) > 0,
            "Progress callback should have been called"
        );
    }

    #[test]
    fn extract_tar_with_hash_calculation() {
        use std::io::Cursor;

        let tar_gz_data = include_bytes!("../../tests/fixtures/test.tar.gz");
        let cursor = Cursor::new(tar_gz_data.to_vec());

        let temp_dir = tempfile::tempdir().unwrap();
        let dest = temp_dir.path().join("output");

        let options = ExtractionOptions::default().hash_strategy(HashStrategy::Blake3);

        let extractor = ArchiveExtractor::Tar(TarExtractor::new(Compression::Gzip));
        let result = extractor.extract(cursor, &dest, &options, None);

        assert!(result.is_ok());
        let report = result.unwrap();
        assert!(
            report.entries.iter().any(|e| e.hash.is_some()),
            "Some entries should have hashes"
        );
    }

    #[test]
    fn extract_actual_zip_file() {
        use std::io::Cursor;
        
        const NAME: &str = "hello_zip.txt";
        const CONTENT:&str = "Hello from ZIP!";

        let zip_data = include_bytes!("../../tests/fixtures/test.zip");
        let cursor = Cursor::new(zip_data.to_vec());

        let temp_dir = tempfile::tempdir().unwrap();
        let dest = temp_dir.path().join("output");

        let options =
            ExtractionOptions::default().permission_strategy(PermissionStrategy::Standard);

        let extractor = ArchiveExtractor::Zip(ZipExtractor);
        let result = extractor.extract(cursor, &dest, &options, None);

        assert!(
            result.is_ok(),
            "Extraction should succeed: {:?}",
            result.err()
        );
        let report = result.unwrap();
        assert!(report.entry_count > 0, "Should have extracted entries");
        assert!(report.total_bytes > 0, "Should have extracted bytes");

        let extracted_file = dest.join(NAME);
        assert!(
            extracted_file.exists(),
            "Extracted file should exist: {:?}",
            extracted_file
        );
        let content = std::fs::read_to_string(&extracted_file).unwrap();
        assert!(content.contains(CONTENT), "Content should match");
    }

    #[test]
    fn extract_zip_with_progress_callback() {
        use std::io::Cursor;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let zip_data = include_bytes!("../../tests/fixtures/test.zip");
        let cursor = Cursor::new(zip_data.to_vec());

        let call_count = Arc::new(AtomicUsize::new(0));
        let call_count_clone = Arc::clone(&call_count);

        let temp_dir = tempfile::tempdir().unwrap();
        let dest = temp_dir.path().join("output");

        let options = ExtractionOptions::default().on_progress(Arc::new(move |_: Progress| {
            call_count_clone.fetch_add(1, Ordering::SeqCst);
        }));

        let extractor = ArchiveExtractor::Zip(ZipExtractor);
        let result = extractor.extract(cursor, &dest, &options, None);

        assert!(result.is_ok());
        assert!(
            call_count.load(Ordering::SeqCst) > 0,
            "Progress callback should have been called"
        );
    }

    #[test]
    fn extract_zip_with_hash_calculation() {
        use std::io::Cursor;

        let zip_data = include_bytes!("../../tests/fixtures/test.zip");
        let cursor = Cursor::new(zip_data.to_vec());

        let temp_dir = tempfile::tempdir().unwrap();
        let dest = temp_dir.path().join("output");

        let options = ExtractionOptions::default().hash_strategy(HashStrategy::Sha256);

        let extractor = ArchiveExtractor::Zip(ZipExtractor);
        let result = extractor.extract(cursor, &dest, &options, None);

        assert!(result.is_ok());
        let report = result.unwrap();
        assert!(
            report.entries.iter().any(|e| e.hash.is_some()),
            "Some entries should have hashes"
        );
    }
}
