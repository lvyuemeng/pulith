//! Serialization backend contract crate.

use serde::Serialize;
use serde::de::DeserializeOwned;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, CodecError>;

#[derive(Debug, Error)]
pub enum CodecError {
    #[error("json codec error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid utf-8 payload: {0}")]
    InvalidUtf8(String),
}

/// Text-oriented structured codec boundary.
pub trait TextCodec {
    fn encode_pretty<T: Serialize>(&self, value: &T) -> Result<String>;
    fn decode_str<T: DeserializeOwned>(&self, input: &str) -> Result<T>;
}

pub fn encode_pretty_vec<C: TextCodec, T: Serialize>(codec: &C, value: &T) -> Result<Vec<u8>> {
    Ok(codec.encode_pretty(value)?.into_bytes())
}

pub fn decode_slice<C: TextCodec, T: DeserializeOwned>(codec: &C, input: &[u8]) -> Result<T> {
    let text =
        std::str::from_utf8(input).map_err(|error| CodecError::InvalidUtf8(error.to_string()))?;
    codec.decode_str(text)
}

/// JSON baseline adapter for structured persistence.
#[derive(Debug, Clone, Copy, Default)]
pub struct JsonTextCodec;

impl TextCodec for JsonTextCodec {
    fn encode_pretty<T: Serialize>(&self, value: &T) -> Result<String> {
        Ok(serde_json::to_string_pretty(value)?)
    }

    fn decode_str<T: DeserializeOwned>(&self, input: &str) -> Result<T> {
        Ok(serde_json::from_str(input)?)
    }
}

/// Compact JSON adapter used for parity/compatibility testing.
#[derive(Debug, Clone, Copy, Default)]
pub struct CompactJsonTextCodec;

impl TextCodec for CompactJsonTextCodec {
    fn encode_pretty<T: Serialize>(&self, value: &T) -> Result<String> {
        Ok(serde_json::to_string(value)?)
    }

    fn decode_str<T: DeserializeOwned>(&self, input: &str) -> Result<T> {
        Ok(serde_json::from_str(input)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::collections::BTreeMap;

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    struct Example {
        schema_version: u32,
        entries: BTreeMap<String, String>,
    }

    #[test]
    fn json_text_codec_round_trip_preserves_data() {
        let mut entries = BTreeMap::new();
        entries.insert("alpha".to_string(), "1".to_string());
        entries.insert("zeta".to_string(), "2".to_string());
        let value = Example {
            schema_version: 1,
            entries,
        };

        let codec = JsonTextCodec;
        let encoded = codec.encode_pretty(&value).unwrap();
        let decoded: Example = codec.decode_str(&encoded).unwrap();

        assert_eq!(decoded, value);
    }

    #[test]
    fn json_text_codec_preserves_btreemap_ordering_in_output() {
        let mut entries = BTreeMap::new();
        entries.insert("zeta".to_string(), "2".to_string());
        entries.insert("alpha".to_string(), "1".to_string());
        let value = Example {
            schema_version: 1,
            entries,
        };

        let codec = JsonTextCodec;
        let encoded = codec.encode_pretty(&value).unwrap();
        let alpha = encoded.find("alpha").unwrap();
        let zeta = encoded.find("zeta").unwrap();
        assert!(alpha < zeta);
    }

    #[test]
    fn helpers_encode_and_decode_bytes() {
        let mut entries = BTreeMap::new();
        entries.insert("alpha".to_string(), "1".to_string());
        let value = Example {
            schema_version: 1,
            entries,
        };

        let codec = JsonTextCodec;
        let encoded = encode_pretty_vec(&codec, &value).unwrap();
        let decoded: Example = decode_slice(&codec, &encoded).unwrap();

        assert_eq!(decoded, value);
    }

    #[test]
    fn codecs_preserve_semantic_parity() {
        let mut entries = BTreeMap::new();
        entries.insert("zeta".to_string(), "2".to_string());
        entries.insert("alpha".to_string(), "1".to_string());
        let value = Example {
            schema_version: 1,
            entries,
        };

        let pretty = JsonTextCodec.encode_pretty(&value).unwrap();
        let compact = CompactJsonTextCodec.encode_pretty(&value).unwrap();

        let pretty_decoded: Example = JsonTextCodec.decode_str(&pretty).unwrap();
        let compact_decoded: Example = CompactJsonTextCodec.decode_str(&compact).unwrap();
        let cross_decoded: Example = JsonTextCodec.decode_str(&compact).unwrap();

        assert_eq!(pretty_decoded, compact_decoded);
        assert_eq!(pretty_decoded, cross_decoded);
    }
}
