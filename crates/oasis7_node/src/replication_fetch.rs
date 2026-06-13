use super::GossipReplicationMessage;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct FetchCommitRequest {
    pub world_id: String,
    pub height: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requester_public_key_hex: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requester_signature_hex: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct FetchCommitResponse {
    pub found: bool,
    pub message: Option<GossipReplicationMessage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct FetchBlobRequest {
    pub content_hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requester_public_key_hex: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requester_signature_hex: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct FetchBlobResponse {
    pub found: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range_offset_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range_complete: Option<bool>,
    #[serde(
        default,
        deserialize_with = "deserialize_optional_hex_or_bytes",
        serialize_with = "serialize_optional_hex_bytes"
    )]
    pub blob: Option<Vec<u8>>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum BlobBytesWire {
    Hex(String),
    Bytes(Vec<u8>),
}

fn serialize_optional_hex_bytes<S>(blob: &Option<Vec<u8>>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match blob {
        Some(bytes) => serializer.serialize_some(&hex::encode(bytes)),
        None => serializer.serialize_none(),
    }
}

fn deserialize_optional_hex_or_bytes<'de, D>(deserializer: D) -> Result<Option<Vec<u8>>, D::Error>
where
    D: Deserializer<'de>,
{
    let Some(wire) = Option::<BlobBytesWire>::deserialize(deserializer)? else {
        return Ok(None);
    };
    match wire {
        BlobBytesWire::Hex(value) => hex::decode(value)
            .map(Some)
            .map_err(serde::de::Error::custom),
        BlobBytesWire::Bytes(bytes) => Ok(Some(bytes)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fetch_blob_response_serializes_blob_as_compact_hex_string() {
        let response = FetchBlobResponse {
            found: true,
            range_offset_bytes: None,
            range_complete: None,
            blob: Some(vec![0, 1, 2, 254, 255]),
        };

        let encoded = serde_json::to_string(&response).expect("encode response");

        assert!(encoded.contains("\"blob\":\"000102feff\""));
        assert!(!encoded.contains("[0,1,2"));
    }

    #[test]
    fn fetch_blob_response_accepts_legacy_json_byte_array() {
        let decoded: FetchBlobResponse =
            serde_json::from_str(r#"{"found":true,"blob":[0,1,2,254,255]}"#)
                .expect("decode legacy response");

        assert_eq!(decoded.blob, Some(vec![0, 1, 2, 254, 255]));
    }
}
