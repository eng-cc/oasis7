use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

pub const WORLD_FEED_SCHEMA_VERSION: &str = "world_feed/v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorldFeedStatus {
    Loading,
    Ready,
    Empty,
    Replay,
    Gap,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorldFeedGapReason {
    CursorGap,
    ReorgEpochChanged,
    CursorInvalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorldFeedUnavailableReason {
    SourceUnavailable,
    SchemaUnsupported,
    PermissionDenied,
}

fn serialize_u64_as_decimal_string<S>(value: &u64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&value.to_string())
}

fn deserialize_u64_from_decimal_string_or_number<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    struct U64DecimalVisitor;

    impl<'de> de::Visitor<'de> for U64DecimalVisitor {
        type Value = u64;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("an unsigned 64-bit integer or decimal string")
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(value)
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            u64::try_from(value).map_err(|_| E::custom("expected a non-negative integer"))
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            value
                .parse::<u64>()
                .map_err(|_| E::custom("expected a decimal u64 string"))
        }
    }

    deserializer.deserialize_any(U64DecimalVisitor)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldFeedEvent {
    #[serde(
        serialize_with = "serialize_u64_as_decimal_string",
        deserialize_with = "deserialize_u64_from_decimal_string_or_number"
    )]
    pub event_seq: u64,
    pub kind: String,
    pub summary: String,
    pub detail: String,
    pub receipt_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldFeedEnvelope {
    pub schema_version: String,
    pub world_id: String,
    #[serde(
        serialize_with = "serialize_u64_as_decimal_string",
        deserialize_with = "deserialize_u64_from_decimal_string_or_number"
    )]
    pub reorg_epoch: u64,
    pub cursor: String,
    pub events: Vec<WorldFeedEvent>,
    pub status: WorldFeedStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gap_reason: Option<WorldFeedGapReason>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unavailable_reason: Option<WorldFeedUnavailableReason>,
    pub snapshot_reload_required: bool,
}
