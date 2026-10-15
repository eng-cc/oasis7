use super::*;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

pub(super) const REQUEST_DOMAIN: &str = "oasis7.endpoint.rpc.request.v1";
pub(super) const RESPONSE_DOMAIN: &str = "oasis7.endpoint.rpc.response.v1";
const MAX_FRAME: usize = 64 * 1024 * 1024;

#[derive(Clone, Serialize, Deserialize)]
pub(super) enum Command {
    Head,
    Freeze(Box<SignedFrozenInput>),
    CompleteFrozen(Box<FrozenCertificate>),
    QueryFrozen(LocalRequestIdentity),
    Lookup(LocalRequestIdentity),
    Prepare(Box<SignedProposal>),
    DecidePrimary(Box<SignedProposal>, Box<DurableReceipt>),
    DecideReplica(
        Box<SignedProposal>,
        Box<DurableReceipt>,
        Box<DurableReceipt>,
    ),
    Finalize(Box<DurabilityEvidence>),
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RequestBody {
    pub version: u32,
    pub trust: FixedTrust,
    pub role: EndpointRole,
    pub challenge: String,
    pub minimum: HeadAnchor,
    pub command: Command,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Request {
    pub body: RequestBody,
    pub signature: String,
}
#[derive(Serialize, Deserialize)]
pub(super) enum Value {
    Head,
    Status(EndpointStatus),
    Frozen(FrozenStatus),
    FrozenReceipt(FrozenReceipt),
    Receipt(DurableReceipt),
    Complete,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ResponseBody {
    pub request_digest: String,
    pub trust: FixedTrust,
    pub role: EndpointRole,
    pub head: HeadAnchor,
    pub result: Result<Value, ProtocolError>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Response {
    pub body: ResponseBody,
    pub signature: String,
}

pub(super) fn check_address(address: SocketAddr) -> Result<(), ProtocolError> {
    if !address.ip().is_loopback() {
        return Err(invalid(
            "endpoint RPC requires loopback encrypted-tunnel termination",
        ));
    }
    Ok(())
}
pub(super) fn configure(stream: &TcpStream, timeout: Duration) -> Result<(), ProtocolError> {
    if timeout.is_zero() || timeout > Duration::from_secs(60) {
        return Err(invalid("RPC timeout must be in (0,60s]"));
    }
    // BSD accept may inherit O_NONBLOCK from a polling listener. Frame reads
    // must use the bounded blocking timeout, rather than drop a not-yet-written request.
    stream.set_nonblocking(false).map_err(io)?;
    stream.set_read_timeout(Some(timeout)).map_err(io)?;
    stream.set_write_timeout(Some(timeout)).map_err(io)
}
pub(super) fn write<T: Serialize>(stream: &mut TcpStream, value: &T) -> Result<(), ProtocolError> {
    let bytes = serde_cbor::to_vec(value).map_err(|e| invalid(e.to_string()))?;
    if bytes.len() > MAX_FRAME {
        return Err(invalid("RPC frame limit"));
    }
    stream
        .write_all(&(bytes.len() as u32).to_be_bytes())
        .map_err(io)?;
    stream.write_all(&bytes).map_err(io)
}
pub(super) fn read<T: serde::de::DeserializeOwned>(
    stream: &mut TcpStream,
) -> Result<T, ProtocolError> {
    let mut length = [0; 4];
    stream.read_exact(&mut length).map_err(io)?;
    let length = u32::from_be_bytes(length) as usize;
    if length == 0 || length > MAX_FRAME {
        return Err(invalid("RPC frame limit"));
    }
    let mut bytes = vec![0; length];
    stream.read_exact(&mut bytes).map_err(io)?;
    serde_cbor::from_slice(&bytes).map_err(|e| invalid(e.to_string()))
}
