use core::marker::PhantomData;
use std::io::{Read, Seek};

use crate::error::Error;
use crate::extract::{EntrySource, PendingEntry, PendingEntryKind};
use crate::{Result, format};

pub struct ZipSource<R: Read + Seek> {
    archive: zip::ZipArchive<R>,
    index: usize,
}

impl<R: Read + Seek> ZipSource<R> {
    pub fn new(reader: R) -> Result<Self> {
        let archive = zip::ZipArchive::new(reader).map_err(|_| Error::Corrupted)?;
        Ok(Self { archive, index: 0 })
    }
}

impl<R: Read + Seek> EntrySource for ZipSource<R> {
    // We define the associated type as the concrete ZipFile type.
    type Reader<'a>
        = zip::read::ZipFile<'a, R>
    where
        Self: 'a;

    fn next_entry(&mut self) -> Option<Result<PendingEntry<'_, Self::Reader<'_>>>> {
        if self.index >= self.archive.len() {
            return None;
        }

        // We borrow the archive mutably here
        let mut file = match self.archive.by_index(self.index) {
            Ok(f) => f,
            Err(_) => return Some(Err(Error::Corrupted)),
        };
        self.index += 1;

        // Metadata extraction
        let raw_path = match file.enclosed_name() {
            Some(p) => p.to_path_buf(),
            None => return Some(Err(Error::InvalidPath)),
        };
        let size = file.size();
        let mode = file.unix_mode();
        let is_dir = file.is_dir();

        let kind = if is_dir {
            PendingEntryKind::Directory
        } else {
            let path_str = raw_path.to_string_lossy();
            if path_str.ends_with(".lnk") {
                let mut content = Vec::new();
                if file.read_to_end(&mut content).is_err() {
                    return Some(Err(Error::Corrupted));
                }
                let target = match String::from_utf8(content) {
                    Ok(s) => s.into(),
                    Err(_) => return Some(Err(Error::InvalidPath)),
                };
                PendingEntryKind::Symlink { target }
            } else {
                // ZERO-COPY: We return the file reader directly!
                PendingEntryKind::File(file)
            }
        };

        Some(Ok(PendingEntry {
            original_path: raw_path,
            size,
            mode,
            kind,
            _marker: PhantomData,
        }))
    }

    fn format(&self) -> format::ArchiveFormat {
        format::ArchiveFormat::Zip
    }
}
