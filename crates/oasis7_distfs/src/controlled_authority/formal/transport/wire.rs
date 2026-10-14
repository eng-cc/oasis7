use super::*;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::{Duration, Instant};
const LIMIT: usize = 64 * 1024 * 1024;
#[derive(Clone, Serialize, Deserialize)]
pub(crate) enum Command {
    Head,
    Query(String),
    Freeze(Box<SignedFormalFrozenV1>, Box<ClosedRecord>),
    CompleteFrozen(Box<FormalFrozenCertificateV1>),
    InstallRecoveryFrozen(Box<FormalFrozenCertificateV1>, Box<ClosedRecord>),
    Prepare(Box<FormalCandidateMaterialV1>),
    Decide(
        String,
        Box<FormalPrepareReceiptV1>,
        Box<FormalPrepareReceiptV1>,
    ),
    InstallProof(Box<FormalDecisionProofV1>),
    PublishQualified(Box<FormalQualifiedCommitProofV1>),
    ArchiveManifest,
    ArchiveObject(String),
    ArchiveChunk {
        id: String,
        offset: u64,
        limit: u32,
    },
    Seal {
        operation: String,
        next_epoch: u64,
        checkpoint: fencing::EndpointCheckpointBindingV1,
    },
    SealDurableSet {
        transition: Box<SignedNormalDurableSetTransitionV1>,
        checkpoint: fencing::EndpointCheckpointBindingV1,
    },
    QueryDurableSet(Box<SignedNormalDurableSetTransitionV1>),
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RequestBody {
    pub schema_version: u32,
    pub world: FormalWorldIdentityV1,
    pub authority_id: String,
    pub instance_digest: String,
    pub challenge: String,
    pub minimum: FormalHeadRefV1,
    pub command: Command,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archive_read_grant: Option<SignedArchiveReadGrantV1>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Request {
    pub body: RequestBody,
    pub signature: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub(crate) enum Value {
    Complete,
    Query(Box<FormalQueryOutcomeV1>),
    Frozen(Box<FormalFrozenReceiptV1>),
    Prepare(Box<FormalPrepareReceiptV1>),
    Final(Box<FormalFinalReceiptV1>),
    ProofComplete(Box<FormalProofCompleteReceiptV1>),
    ArchiveManifest(Box<SignedFormalArchiveTransferV1>),
    ArchiveObject(Option<Vec<u8>>),
    ArchiveChunk(Option<FormalArchiveChunkV1>),
    Seal(Box<fencing::EpochSealReceiptV1>),
    DurableSetSeal(Option<Box<fencing::EpochSealReceiptV1>>),
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ResponseBody {
    pub schema_version: u32,
    pub request_digest: String,
    pub challenge: String,
    pub world: FormalWorldIdentityV1,
    pub authority_id: String,
    pub instance_digest: String,
    pub minimum: FormalHeadRefV1,
    pub head: FormalHeadRefV1,
    pub checkpoint: fencing::EndpointCheckpointBindingV1,
    pub checkpoint_bytes: Vec<u8>,
    pub ancestry: Vec<Vec<u8>>,
    pub provider: fencing::MonotonicProviderEvidenceV1,
    pub result: Result<Value, FormalError>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Response {
    pub body: ResponseBody,
    pub signature: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protected_signature: Option<fencing::ProtectedEndpointSignatureV1>,
}
pub(super) fn address(address: SocketAddr) -> Result<(), FormalError> {
    if !address.ip().is_loopback() {
        return Err(invalid(
            "formal RPC requires authenticated encrypted-tunnel loopback termination",
        ));
    }
    Ok(())
}
pub(super) fn configure(stream: &TcpStream, timeout: Duration) -> Result<(), FormalError> {
    if timeout.is_zero() || timeout > Duration::from_secs(60) {
        return Err(invalid("formal RPC timeout in (0,60s]"));
    }
    stream.set_nonblocking(false).map_err(io)?;
    // A frame's length and CBOR body are separate writes. Avoid delaying the
    // body behind the small length packet during multi-object cold restore.
    stream.set_nodelay(true).map_err(io)?;
    stream.set_read_timeout(Some(timeout)).map_err(io)?;
    stream.set_write_timeout(Some(timeout)).map_err(io)
}
pub(super) fn io(error: impl std::fmt::Display) -> FormalError {
    FormalError::Io(error.to_string())
}
pub(super) fn write<T: Serialize>(stream: &mut TcpStream, value: &T) -> Result<(), FormalError> {
    let bytes = serde_cbor::to_vec(value).map_err(|e| invalid(e.to_string()))?;
    if bytes.len() > LIMIT {
        return Err(invalid("formal RPC bounded frame"));
    }
    stream
        .write_all(&(bytes.len() as u32).to_be_bytes())
        .and_then(|_| stream.write_all(&bytes))
        .map_err(io)
}
pub(super) fn read<T: serde::de::DeserializeOwned>(
    stream: &mut TcpStream,
) -> Result<T, FormalError> {
    let mut length = [0; 4];
    stream.read_exact(&mut length).map_err(io)?;
    let length = u32::from_be_bytes(length) as usize;
    if length == 0 || length > LIMIT {
        return Err(invalid("formal RPC bounded frame"));
    }
    let mut bytes = vec![0; length];
    stream.read_exact(&mut bytes).map_err(io)?;
    serde_cbor::from_slice(&bytes).map_err(|e| invalid(e.to_string()))
}

fn remaining(deadline: Instant) -> Result<Duration, FormalError> {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|time| !time.is_zero())
        .ok_or_else(|| io("formal RPC frame deadline expired"))
}
fn read_exact_before(
    stream: &mut TcpStream,
    mut bytes: &mut [u8],
    deadline: Instant,
) -> Result<(), FormalError> {
    while !bytes.is_empty() {
        stream
            .set_read_timeout(Some(remaining(deadline)?))
            .map_err(io)?;
        let count = stream.read(bytes).map_err(io)?;
        if count == 0 {
            return Err(io("formal RPC frame ended early"));
        }
        bytes = &mut bytes[count..];
    }
    Ok(())
}
pub(super) fn read_before_deadline<T: serde::de::DeserializeOwned>(
    stream: &mut TcpStream,
    deadline: Instant,
) -> Result<T, FormalError> {
    let mut length = [0; 4];
    read_exact_before(stream, &mut length, deadline)?;
    let length = u32::from_be_bytes(length) as usize;
    if length == 0 || length > LIMIT {
        return Err(invalid("formal RPC bounded frame"));
    }
    let mut bytes = vec![0; length];
    read_exact_before(stream, &mut bytes, deadline)?;
    remaining(deadline)?;
    serde_cbor::from_slice(&bytes).map_err(|error| invalid(error.to_string()))
}
pub(super) fn write_before_deadline<T: Serialize>(
    stream: &mut TcpStream,
    value: &T,
    deadline: Instant,
) -> Result<(), FormalError> {
    let bytes = serde_cbor::to_vec(value).map_err(|error| invalid(error.to_string()))?;
    if bytes.len() > LIMIT {
        return Err(invalid("formal RPC bounded frame"));
    }
    let header = (bytes.len() as u32).to_be_bytes();
    for mut remaining_bytes in [&header[..], &bytes[..]] {
        while !remaining_bytes.is_empty() {
            stream
                .set_write_timeout(Some(remaining(deadline)?))
                .map_err(io)?;
            let count = stream.write(remaining_bytes).map_err(io)?;
            if count == 0 {
                return Err(io("formal RPC frame write ended early"));
            }
            remaining_bytes = &remaining_bytes[count..];
        }
    }
    Ok(())
}

#[cfg(test)]
mod deadline_tests {
    use super::*;
    #[test]
    fn formal_rpc_frame_deadline_is_not_reset_by_slow_fragments() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let sender = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let payload = serde_cbor::to_vec(&serde_cbor::Value::Bytes(vec![7; 16])).unwrap();
            let mut frame = (payload.len() as u32).to_be_bytes().to_vec();
            frame.extend(payload);
            for byte in frame {
                if stream.write_all(&[byte]).is_err() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(15));
            }
        });
        let mut stream = TcpStream::connect(address).unwrap();
        let result = read_before_deadline::<serde_cbor::Value>(
            &mut stream,
            Instant::now() + Duration::from_millis(60),
        );
        assert!(
            result.is_err(),
            "a slow peer must not renew the absolute frame deadline"
        );
        drop(stream);
        sender.join().unwrap();
    }
}
