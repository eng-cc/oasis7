use super::{wire::*, *};
use std::net::TcpListener;
use std::time::Duration;

/// Sequential server retains the storage lock for its entire lifetime. An
/// authentication failure produces no endpoint data or signed result.
pub struct EndpointServer {
    endpoint: FileEndpoint,
    timeout: Duration,
}
impl EndpointServer {
    pub fn new(endpoint: FileEndpoint, timeout: Duration) -> Result<Self, ProtocolError> {
        if timeout.is_zero() || timeout > Duration::from_secs(60) {
            return Err(invalid("RPC timeout must be in (0,60s]"));
        }
        Ok(Self { endpoint, timeout })
    }
    pub fn serve_one(&mut self, listener: &TcpListener) -> Result<(), ProtocolError> {
        check_address(listener.local_addr().map_err(io)?)?;
        let (mut stream, peer) = listener.accept().map_err(io)?;
        check_address(peer)?;
        configure(&stream, self.timeout)?;
        let request: Request = read(&mut stream)?;
        let body = &request.body;
        if body.version != 1
            || &body.trust != self.endpoint.trust()
            || body.role != self.endpoint.role()
        {
            return Err(invalid("RPC endpoint identity/version mismatch"));
        }
        crypto::hash_hex(&body.challenge)?;
        crypto::verify(
            REQUEST_DOMAIN,
            body,
            &request.signature,
            &body.trust.writer_key,
        )?;
        let result = self
            .endpoint
            .verify_rpc_minimum(&body.minimum)
            .and_then(|()| self.dispatch(body.command.clone()));
        let response = ResponseBody {
            request_digest: crypto::hash(REQUEST_DOMAIN, body)?,
            trust: self.endpoint.trust().clone(),
            role: self.endpoint.role(),
            head: self.endpoint.head(),
            result,
        };
        let signature = self.endpoint.sign_rpc_response(&response)?;
        write(
            &mut stream,
            &Response {
                body: response,
                signature,
            },
        )
    }
    fn dispatch(&mut self, command: Command) -> Result<Value, ProtocolError> {
        match command {
            Command::Freeze(input) => self.endpoint.freeze_input(&input).map(Value::FrozenReceipt),
            Command::CompleteFrozen(certificate) => self
                .endpoint
                .complete_frozen(&certificate)
                .map(|()| Value::Complete),
            Command::QueryFrozen(request) => {
                self.endpoint.query_frozen(&request).map(Value::Frozen)
            }
            Command::Head => Ok(Value::Head),
            Command::Lookup(request) => self.endpoint.lookup(&request).map(Value::Status),
            Command::Prepare(proposal) => self.endpoint.prepare(&proposal).map(Value::Receipt),
            Command::DecidePrimary(proposal, prepared) => self
                .endpoint
                .decide_primary(&proposal, &prepared)
                .map(Value::Receipt),
            Command::DecideReplica(proposal, prepared, primary) => self
                .endpoint
                .decide_replica(&proposal, &prepared, &primary)
                .map(Value::Receipt),
            Command::Finalize(evidence) => {
                self.endpoint.finalize(&evidence).map(|()| Value::Complete)
            }
        }
    }
}
