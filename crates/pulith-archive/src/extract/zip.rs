use std::io::{Read, Seek};

use crate::entry::EntryKind;
use crate::error::Error;
use crate::extract::{EntrySource, PendingEntry};
use crate::{Result, format};

/// Zip archive entry source.
pub struct ZipSource<R: Read + Seek> {
    archive: zip::ZipArchive<R>,
}

impl<R: Read + Seek> ZipSource<R> {
    pub fn new(reader: R) -> Result<Self> {
        let archive = zip::ZipArchive::new(reader).map_err(|_| Error::Corrupted)?;
        Ok(Self { archive })
    }
}

impl<R: Read + Seek> EntrySource for ZipSource<R> {
    fn entries(&mut self) -> Result<Box<dyn Iterator<Item = Result<PendingEntry>> + '_>> {
        Ok(Box::new(ZipEntries::new(&mut self.archive)))
    }

    fn format(&self) -> format::ArchiveFormat {
        format::ArchiveFormat::Zip
    }
}

/// Iterator over zip archive entries.
struct ZipEntries<'a, R: Read + Seek> {
    archive: &'a mut zip::ZipArchive<R>,
    index: usize,
}

impl<'a, R: Read + Seek> ZipEntries<'a, R> {
    fn new(archive: &'a mut zip::ZipArchive<R>) -> Self {
        Self { archive, index: 0 }
    }
}

impl<'a, R: Read + Seek> Iterator for ZipEntries<'a, R> {
    type Item = Result<PendingEntry>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.archive.len() {
            return None;
        }

        let mut file = match self.archive.by_index(self.index) {
            Ok(f) => f,
            Err(_) => return Some(Err(Error::Corrupted)),
        };
        self.index += 1;

        let raw_path = match file.enclosed_name() {
            Some(p) => p.to_path_buf(),
            None => return Some(Err(Error::InvalidPath)),
        };

        let size = file.size();
        let mode = file.unix_mode();

        let kind = if file.is_dir() {
            EntryKind::Directory
        } else {
            let symlink_indicator = raw_path.as_os_str().to_string_lossy();
            if symlink_indicator.ends_with(".lnk") || symlink_indicator.contains(".lnk") {
                let mut content = Vec::new();
                if file.read_to_end(&mut content).is_err() {
                    return Some(Err(Error::Corrupted));
                }
                let target = match String::from_utf8(content) {
                    Ok(s) => s.into(),
                    Err(_) => return Some(Err(Error::InvalidPath)),
                };
                EntryKind::Symlink { target }
            } else {
                EntryKind::File
            }
        };

        let reader: Option<Box<dyn Read>> = if matches!(kind, EntryKind::File) {
            let mut content = Vec::with_capacity(size as usize);
            if let Err(e) = file.read_to_end(&mut content) {
                return Some(Err(Error::from(e)));
            }
            Some(Box::new(std::io::Cursor::new(content)))
        } else {
            None
        };

        Some(Ok(PendingEntry {
            original_path: raw_path,
            size,
            mode,
            kind,
            reader,
        }))
    }
}
