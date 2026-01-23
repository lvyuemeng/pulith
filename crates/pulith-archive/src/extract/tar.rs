use std::io::Read;

use crate::entry::EntryKind;
use crate::error::Error;
use crate::extract::{EntrySource, PendingEntry};
use crate::format::{Decoder, TarCompress};
use crate::{Result, format};

/// Tar archive entry source.
pub struct TarSource<R: Read> {
    archive: tar::Archive<Decoder<R>>,
}

impl<R: Read> TarSource<R> {
    pub fn new(reader: R, codec: TarCompress) -> Result<Self> {
        let decoder = codec.decoder(reader)?;
        let archive = tar::Archive::new(decoder);
        Ok(Self { archive })
    }
}

impl<R: Read> EntrySource for TarSource<R> {
    fn entries(&mut self) -> Result<Box<dyn Iterator<Item = Result<PendingEntry>> + '_>> {
        let iter = self.archive.entries()?.map(move |result| {
            let mut entry = result.map_err(|_| Error::Corrupted)?;

            let raw_path = entry.path()?.into_owned();
            let header = entry.header();

            let size = header.size().unwrap_or(0);
            let mode = header.mode().ok();
            let entry_type = header.entry_type();

            let kind = match entry_type {
                t if t.is_dir() => EntryKind::Directory,
                t if t.is_symlink() => {
                    let target = entry.link_name()?.ok_or(Error::InvalidPath)?.into_owned();
                    EntryKind::Symlink { target }
                }
                _ => EntryKind::File,
            };

            let reader: Option<Box<dyn Read>> = if matches!(kind, EntryKind::File) {
                let mut content = Vec::with_capacity(size as usize);
                if entry.read_to_end(&mut content).is_err() {
                    return Err(Error::Corrupted);
                }
                Some(Box::new(std::io::Cursor::new(content)))
            } else {
                None
            };

            Ok(PendingEntry {
                original_path: raw_path,
                size,
                mode,
                kind,
                reader,
            })
        });

        Ok(Box::new(iter))
    }

    fn format(&self) -> format::ArchiveFormat {
        format::ArchiveFormat::Tar(TarCompress::None)
    }
}
