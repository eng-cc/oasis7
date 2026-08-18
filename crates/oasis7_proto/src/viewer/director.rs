use serde::{Deserialize, Serialize};

/// Ephemeral, read-only capability for opening the Viewer Director diagnostics surface.
///
/// This grant is intentionally separate from player command/auth proofs.  It does not
/// authorize gameplay, ownership, prompt control, or any other mutation; it only binds a
/// short-lived diagnostics visibility decision to a server session epoch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectorCapabilityGrant {
    pub version: u8,
    pub action: String,
    pub audience: String,
    pub scope: String,
    pub player_id: String,
    pub player_public_key: String,
    pub server: String,
    pub session_epoch: u64,
    pub nonce: String,
    pub issued_at_unix_ms: u64,
    pub expires_at_unix_ms: u64,
    pub signer_public_key: String,
    pub signature: String,
}

pub const DIRECTOR_CAPABILITY_GRANT_VERSION: u8 = 1;
pub const DIRECTOR_CAPABILITY_DOMAIN: &str = "awdirectorgrant:v1";
pub const DIRECTOR_CAPABILITY_ACTION: &str = "director_open";
pub const DIRECTOR_CAPABILITY_AUDIENCE: &str = "viewer_director";
pub const DIRECTOR_CAPABILITY_SCOPE: &str = "diagnostics_read";
pub const DIRECTOR_CAPABILITY_SIGNATURE_V1_PREFIX: &str = "awdirectorgrant:v1:";
pub const DIRECTOR_CAPABILITY_MAX_TTL_MS: u64 = 60_000;
