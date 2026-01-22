use std::io::Read;

use crate::data::archive::Compression;

pub fn wrap_reader<R: Read + 'static>(
    reader: R,
    codec: Compression,
) -> Result<Box<dyn Read>, crate::Error> {
    match codec {
        Compression::None => Ok(Box::new(reader)),
        Compression::Gzip => Ok(Box::new(flate2::read::GzDecoder::new(reader))),
        #[cfg(feature = "xz")]
        Compression::Xz => Ok(Box::new(xz2::read::XzDecoder::new(reader))),
        #[cfg(not(feature = "xz"))]
        Compression::Xz => Err(crate::error::Error::UnsupportedFormat),
        #[cfg(feature = "zstd")]
        Compression::Zstd => {
            let decoder =
                zstd::stream::Decoder::new(reader).map_err(|_| crate::error::Error::Corrupted)?;
            Ok(Box::new(decoder))
        }
        #[cfg(not(feature = "zstd"))]
        Compression::Zstd => Err(crate::error::Error::UnsupportedFormat),
    }
}
