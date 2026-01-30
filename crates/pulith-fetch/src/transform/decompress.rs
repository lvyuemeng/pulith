//! Stream decompression functionality.
//!
//! This module provides stream transformation for decompressing
//! downloaded content on the fly.

use crate::error::{Error, Result};
use std::io::{Read, Write};

/// Compression types supported by the fetcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionType {
    /// No compression
    None,
    /// Gzip compression
    Gzip,
    /// Deflate compression
    Deflate,
    /// Brotli compression (feature-gated)
    #[cfg(feature = "brotli")]
    Brotli,
}

impl CompressionType {
    /// Detect compression type from Content-Encoding header value.
    pub fn from_encoding(encoding: &str) -> Self {
        match encoding.to_lowercase().as_str() {
            "gzip" | "x-gzip" => CompressionType::Gzip,
            "deflate" => CompressionType::Deflate,
            #[cfg(feature = "brotli")]
            "br" => CompressionType::Brotli,
            _ => CompressionType::None,
        }
    }

    /// Get the Content-Encoding header value for this compression type.
    pub fn as_encoding(self) -> &'static str {
        match self {
            CompressionType::None => "identity",
            CompressionType::Gzip => "gzip",
            CompressionType::Deflate => "deflate",
            #[cfg(feature = "brotli")]
            CompressionType::Brotli => "br",
        }
    }
}

/// Error type for stream transformations.
#[derive(Debug, thiserror::Error)]
pub enum TransformError {
    #[error("Transformation error: {0}")]
    Transform(String),
    #[error("Invalid compressed data: {0}")]
    InvalidData(String),
    #[error("Unsupported compression type: {0:?}")]
    UnsupportedType(CompressionType),
}

/// Stream transform trait for decompression.
pub trait StreamTransform {
    /// Transform the input bytes.
    fn transform(&mut self, input: &[u8]) -> Result<Vec<u8>>;

    /// Finalize the transformation (for stream-based decoders).
    fn finalize(&mut self) -> Result<Vec<u8>> {
        Ok(Vec::new())
    }

    /// Reset the transformer state.
    fn reset(&mut self) -> Result<()>;
}

/// Gzip decompressor implementation.
pub struct GzipDecoder {
    decoder: Option<flate2::read::GzDecoder<std::io::Cursor<Vec<u8>>>>,
    buffer: Vec<u8>,
}

impl GzipDecoder {
    /// Create a new Gzip decoder.
    pub fn new() -> Self {
        Self {
            decoder: None,
            buffer: Vec::new(),
        }
    }
}

impl StreamTransform for GzipDecoder {
    fn transform(&mut self, input: &[u8]) -> Result<Vec<u8>> {
        self.buffer.extend_from_slice(input);

        if self.decoder.is_none() {
            self.decoder = Some(flate2::read::GzDecoder::new(std::io::Cursor::new(
                self.buffer.clone(),
            )));
        }

        let mut output = Vec::new();
        if let Some(ref mut decoder) = self.decoder {
            decoder
                .read_to_end(&mut output)
                .map_err(|e| Error::Transform(TransformError::InvalidData(e.to_string())))?;
        }

        Ok(output)
    }

    fn finalize(&mut self) -> Result<Vec<u8>> {
        if self.decoder.is_none() && !self.buffer.is_empty() {
            // Try to decode remaining data
            self.transform(&[])
        } else {
            Ok(Vec::new())
        }
    }

    fn reset(&mut self) -> Result<()> {
        self.decoder = None;
        self.buffer.clear();
        Ok(())
    }
}

/// Deflate decompressor implementation.
pub struct DeflateDecoder {
    decoder: Option<flate2::read::DeflateDecoder<std::io::Cursor<Vec<u8>>>>,
    buffer: Vec<u8>,
}

impl DeflateDecoder {
    /// Create a new Deflate decoder.
    pub fn new() -> Self {
        Self {
            decoder: None,
            buffer: Vec::new(),
        }
    }
}

impl StreamTransform for DeflateDecoder {
    fn transform(&mut self, input: &[u8]) -> Result<Vec<u8>> {
        self.buffer.extend_from_slice(input);

        if self.decoder.is_none() {
            self.decoder = Some(flate2::read::DeflateDecoder::new(std::io::Cursor::new(
                self.buffer.clone(),
            )));
        }

        let mut output = Vec::new();
        if let Some(ref mut decoder) = self.decoder {
            decoder
                .read_to_end(&mut output)
                .map_err(|e| Error::Transform(TransformError::InvalidData(e.to_string())))?;
        }

        Ok(output)
    }

    fn finalize(&mut self) -> Result<Vec<u8>> {
        if self.decoder.is_none() && !self.buffer.is_empty() {
            // Try to decode remaining data
            self.transform(&[])
        } else {
            Ok(Vec::new())
        }
    }

    fn reset(&mut self) -> Result<()> {
        self.decoder = None;
        self.buffer.clear();
        Ok(())
    }
}

/// Brotli decompressor implementation (feature-gated).
#[cfg(feature = "brotli")]
pub struct BrotliDecoder {
    decoder: Option<brotli::DecompressorWriter<std::io::Cursor<Vec<u8>>>>,
    buffer: Vec<u8>,
}

#[cfg(feature = "brotli")]
impl BrotliDecoder {
    /// Create a new Brotli decoder.
    pub fn new() -> Self {
        Self {
            decoder: None,
            buffer: Vec::new(),
        }
    }
}

#[cfg(feature = "brotli")]
impl StreamTransform for BrotliDecoder {
    fn transform(&mut self, input: &[u8]) -> Result<Vec<u8>> {
        self.buffer.extend_from_slice(input);

        if self.decoder.is_none() {
            self.decoder = Some(brotli::DecompressorWriter::new(
                std::io::Cursor::new(Vec::new()),
                4096, // buffer size
            ));
        }

        let mut output = Vec::new();
        if let Some(ref mut decoder) = self.decoder {
            decoder
                .write_all(input)
                .map_err(|e| Error::Transform(TransformError::InvalidData(e.to_string())))?;
            decoder
                .flush()
                .map_err(|e| Error::Transform(TransformError::InvalidData(e.to_string())))?;

            // Get the decompressed data
            if let Some(cursor) = decoder.get_mut() {
                output = cursor.get_ref().clone();
            }
        }

        Ok(output)
    }

    fn finalize(&mut self) -> Result<Vec<u8>> {
        if let Some(ref mut decoder) = self.decoder {
            decoder
                .finish()
                .map_err(|e| Error::Transform(TransformError::InvalidData(e.to_string())))?;

            let mut output = Vec::new();
            if let Some(cursor) = decoder.get_mut() {
                output = cursor.get_ref().clone();
            }
            Ok(output)
        } else {
            Ok(Vec::new())
        }
    }

    fn reset(&mut self) -> Result<()> {
        self.decoder = None;
        self.buffer.clear();
        Ok(())
    }
}

/// Factory function to create appropriate decoder for compression type.
pub fn create_decoder(compression_type: CompressionType) -> Result<Box<dyn StreamTransform>> {
    match compression_type {
        CompressionType::None => Err(Error::Transform(TransformError::UnsupportedType(
            CompressionType::None,
        ))),
        CompressionType::Gzip => Ok(Box::new(GzipDecoder::new())),
        CompressionType::Deflate => Ok(Box::new(DeflateDecoder::new())),
        #[cfg(feature = "brotli")]
        CompressionType::Brotli => Ok(Box::new(BrotliDecoder::new())),
    }
}

/// Convenience function to decompress data in one go.
pub fn decompress(data: &[u8], compression_type: CompressionType) -> Result<Vec<u8>> {
    let mut decoder = create_decoder(compression_type)?;
    let result = decoder.transform(data)?;
    let final_data = decoder.finalize()?;

    // Combine result and final data
    let mut output = result;
    output.extend_from_slice(&final_data);

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::DeflateEncoder;
    use flate2::{write::GzEncoder, Compression as FlateCompression};

    fn create_gzip_data(data: &[u8]) -> Vec<u8> {
        let mut encoder = GzEncoder::new(Vec::new(), FlateCompression::default());
        encoder.write_all(data).unwrap();
        encoder.finish().unwrap()
    }

    fn create_deflate_data(data: &[u8]) -> Vec<u8> {
        let mut encoder = DeflateEncoder::new(Vec::new(), FlateCompression::default());
        encoder.write_all(data).unwrap();
        encoder.finish().unwrap()
    }

    #[test]
    fn test_compression_type_detection() {
        assert_eq!(
            CompressionType::from_encoding("gzip"),
            CompressionType::Gzip
        );
        assert_eq!(
            CompressionType::from_encoding("GZIP"),
            CompressionType::Gzip
        );
        assert_eq!(
            CompressionType::from_encoding("x-gzip"),
            CompressionType::Gzip
        );
        assert_eq!(
            CompressionType::from_encoding("deflate"),
            CompressionType::Deflate
        );
        assert_eq!(
            CompressionType::from_encoding("unknown"),
            CompressionType::None
        );
        assert_eq!(CompressionType::from_encoding(""), CompressionType::None);
    }

    #[test]
    fn test_gzip_decompression() {
        let original = b"Hello, World! This is a test string for gzip compression.";
        let compressed = create_gzip_data(original);

        let mut decoder = GzipDecoder::new();
        let decompressed = decoder.transform(&compressed).unwrap();

        assert_eq!(decompressed, original);
    }

    #[test]
    fn test_deflate_decompression() {
        let original = b"Hello, World! This is a test string for deflate compression.";
        let compressed = create_deflate_data(original);

        let mut decoder = DeflateDecoder::new();
        let decompressed = decoder.transform(&compressed).unwrap();

        assert_eq!(decompressed, original);
    }

    #[test]
    fn test_convenience_function() {
        let original = b"Test data for convenience function.";
        let gzip_data = create_gzip_data(original);
        let deflate_data = create_deflate_data(original);

        // Test gzip
        let decompressed = decompress(&gzip_data, CompressionType::Gzip).unwrap();
        assert_eq!(decompressed, original);

        // Test deflate
        let decompressed = decompress(&deflate_data, CompressionType::Deflate).unwrap();
        assert_eq!(decompressed, original);
    }

    #[test]
    fn test_decoder_factory() {
        let gzip_decoder = create_decoder(CompressionType::Gzip);
        assert!(gzip_decoder.is_ok());

        let deflate_decoder = create_decoder(CompressionType::Deflate);
        assert!(deflate_decoder.is_ok());

        let none_decoder = create_decoder(CompressionType::None);
        assert!(none_decoder.is_err());
    }

    #[test]
    fn test_invalid_gzip_data() {
        let invalid_data = b"This is not valid gzip data";
        let mut decoder = GzipDecoder::new();

        let result = decoder.transform(invalid_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_deflate_data() {
        let invalid_data = b"This is not valid deflate data";
        let mut decoder = DeflateDecoder::new();

        let result = decoder.transform(invalid_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_decoder_reset() {
        let original = b"Test data for reset.";
        let compressed = create_gzip_data(original);

        let mut decoder = GzipDecoder::new();

        // First decode
        let result1 = decoder.transform(&compressed).unwrap();
        assert_eq!(result1, original);

        // Reset and decode again
        decoder.reset().unwrap();
        let result2 = decoder.transform(&compressed).unwrap();
        assert_eq!(result2, original);
    }

    #[test]
    fn test_empty_data() {
        let mut decoder = GzipDecoder::new();
        // Empty data should not error, but may return empty result
        let result = decoder.transform(&[]);
        // Empty input without any gzip header will error, which is expected
        assert!(result.is_err() || result.unwrap().is_empty());
    }

    #[test]
    fn test_large_data() {
        let original: Vec<u8> = (0..10000).map(|i| (i % 256) as u8).collect();
        let compressed = create_gzip_data(&original);

        let mut decoder = GzipDecoder::new();
        let decompressed = decoder.transform(&compressed).unwrap();

        assert_eq!(decompressed, original);
    }
}
