use core::marker::PhantomData;
use std::io::Read;

use crate::error::Error;
use crate::extract::{EntrySource, PendingEntry, PendingEntryKind};
use crate::format::{Decoder, TarCompress};
use crate::{Result, format};

// TarSource now holds the iterator, which manages the stream state.
pub struct TarArchive<R: Read + Sync> {
    archive: tar::Archive<R>,
}

impl<R: Read + Sync + Send> TarArchive<Decoder<R>> {
    // You construct this from an existing archive
    pub fn new(reader: R, codec: TarCompress) -> Result<Self> {
        let reader = codec.decoder(reader)?;
        let archive = tar::Archive::new(reader);
        Ok(Self { archive })
    }

    pub fn entries(&mut self) -> Result<TarSource<'_,Decoder<R>>> {
        Ok(TarSource {
            entries: self.archive.entries()?,
        })
    }
}

pub struct TarSource<'a, R: 'a + Read> {
    entries: tar::Entries<'a, R>,
}

impl<'a, R: Read + 'a> EntrySource for TarSource<'a, R> {
    // FIX: Ignore the lifetime 'b provided by the GAT.
    // Explicitly use 'a to match the actual lifetime of items coming from the tar iterator.
    type Reader<'b>
        = tar::Entry<'a, R>
    where
        Self: 'b;

    fn next_entry(&mut self) -> Option<Result<PendingEntry<'_, Self::Reader<'_>>>> {
        // Get the next entry from the underlying iterator
        let entry_result = self.entries.next()?;
        let entry = match entry_result {
            Ok(e) => e,
            Err(_) => return Some(Err(Error::Corrupted)),
        };

        let raw_path = match entry.path() {
            Ok(p) => p.into_owned(),
            Err(_) => return Some(Err(Error::InvalidPath)),
        };

        let header = entry.header();
        let size = header.size().unwrap_or(0);
        let mode = header.mode().ok();
        let entry_type = header.entry_type();

        let kind = if entry_type.is_dir() {
            PendingEntryKind::Directory
        } else if entry_type.is_symlink() {
            let target = match entry.link_name() {
                Ok(Some(t)) => t.into_owned(),
                _ => return Some(Err(Error::InvalidPath)),
            };
            PendingEntryKind::Symlink { target }
        } else {
            PendingEntryKind::File(entry)
        };

        Some(Ok(PendingEntry {
            // Adjust field names to match your struct definition
            // (Assuming 'original_path' based on your snippet)
            original_path: raw_path,
            size,
            mode,
            kind,
            _marker: PhantomData,
        }))
    }

    fn format(&self) -> format::ArchiveFormat {
        format::ArchiveFormat::Tar(TarCompress::None)
    }
}
