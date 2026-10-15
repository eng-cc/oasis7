use super::{wire::*, *};
use ed25519_dalek::SigningKey;
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;
#[cfg(unix)]
fn challenge() -> Result<String, ProtocolError> {
    use std::io::Read;
    let mut nonce = [0u8; 32];
    std::fs::File::open("/dev/urandom")
        .map_err(io)?
        .read_exact(&mut nonce)
        .map_err(io)?;
    Ok(hex::encode(nonce))
}
#[cfg(not(unix))]
fn challenge() -> Result<String, ProtocolError> {
    Err(ProtocolError::UnsupportedPlatform)
}

pub struct RemoteEndpoint {
    address: SocketAddr,
    trust: FixedTrust,
    role: EndpointRole,
    writer: SigningKey,
    head: HeadAnchor,
    timeout: Duration,
}
impl RemoteEndpoint {
    pub fn connect(
        address: SocketAddr,
        trust: FixedTrust,
        role: EndpointRole,
        writer: SigningKey,
        minimum: HeadAnchor,
        timeout: Duration,
    ) -> Result<Self, ProtocolError> {
        check_address(address)?;
        trust.validate()?;
        crypto::hash_hex(&minimum.decision_hash)?;
        if hex::encode(writer.verifying_key().to_bytes()) != trust.writer_key {
            return Err(invalid("RPC writer signer mismatch"));
        }
        if minimum.position == 0 && minimum != trust.genesis_anchor()? {
            return Err(invalid("false RPC genesis anchor"));
        }
        if timeout.is_zero() || timeout > Duration::from_secs(60) {
            return Err(invalid("RPC timeout must be in (0,60s]"));
        }
        let mut endpoint = Self {
            address,
            trust,
            role,
            writer,
            head: minimum,
            timeout,
        };
        match endpoint.call(Command::Head)? {
            Value::Head => Ok(endpoint),
            _ => Err(invalid("RPC head result type")),
        }
    }
    pub(super) fn call(&mut self, command: Command) -> Result<Value, ProtocolError> {
        let challenge = challenge()?;
        let body = RequestBody {
            version: 1,
            trust: self.trust.clone(),
            role: self.role,
            challenge,
            minimum: self.head.clone(),
            command,
        };
        let expected = crypto::hash(REQUEST_DOMAIN, &body)?;
        let signature = crypto::sign(REQUEST_DOMAIN, &body, &self.writer)?;
        let mut stream = TcpStream::connect_timeout(&self.address, self.timeout).map_err(io)?;
        configure(&stream, self.timeout)?;
        write(&mut stream, &Request { body, signature })?;
        let response: Response = read(&mut stream).map_err(|error| match error {
            ProtocolError::Io(_) => error,
            _ => ProtocolError::Poisoned,
        })?;
        let body = response.body;
        if body.request_digest != expected || body.trust != self.trust || body.role != self.role {
            return Err(ProtocolError::Poisoned);
        }
        crypto::verify(
            RESPONSE_DOMAIN,
            &body,
            &response.signature,
            self.trust.endpoint(self.role).1,
        )?;
        crypto::hash_hex(&body.head.decision_hash)?;
        if body.head.position < self.head.position
            || (body.head.position == self.head.position
                && (body.head.decision_hash != self.head.decision_hash
                    || (self.head.qualified && !body.head.qualified)))
        {
            return Err(ProtocolError::Poisoned);
        }
        let result = body.result?;
        self.head = body.head;
        Ok(result)
    }
    fn receipt(&mut self, command: Command) -> Result<DurableReceipt, ProtocolError> {
        match self.call(command)? {
            Value::Receipt(r) => Ok(r),
            _ => Err(ProtocolError::Poisoned),
        }
    }
}
impl DurableEndpoint for RemoteEndpoint {
    fn freeze_input(&mut self, input: &SignedFrozenInput) -> Result<FrozenReceipt, ProtocolError> {
        match self.call(Command::Freeze(Box::new(input.clone())))? {
            Value::FrozenReceipt(receipt) => {
                receipt
                    .verify(input, self.role)
                    .map_err(|_| ProtocolError::Poisoned)?;
                Ok(receipt)
            }
            _ => Err(ProtocolError::Poisoned),
        }
    }
    fn complete_frozen(&mut self, certificate: &FrozenCertificate) -> Result<(), ProtocolError> {
        match self.call(Command::CompleteFrozen(Box::new(certificate.clone())))? {
            Value::Complete => Ok(()),
            _ => Err(ProtocolError::Poisoned),
        }
    }
    fn query_frozen(
        &mut self,
        request: &LocalRequestIdentity,
    ) -> Result<FrozenStatus, ProtocolError> {
        match self.call(Command::QueryFrozen(request.clone()))? {
            Value::Frozen(status) => {
                if let FrozenStatus::Reserved(reservation) = &status {
                    if &reservation.input.body.request != request {
                        return Err(ProtocolError::Poisoned);
                    }
                    reservation
                        .validate(&self.trust, self.role)
                        .map_err(|_| ProtocolError::Poisoned)?;
                }
                Ok(status)
            }
            _ => Err(ProtocolError::Poisoned),
        }
    }

    fn role(&self) -> EndpointRole {
        self.role
    }
    fn trust(&self) -> &FixedTrust {
        &self.trust
    }
    fn head(&self) -> HeadAnchor {
        self.head.clone()
    }
    fn storage_identity(&self) -> Result<String, ProtocolError> {
        Ok(format!("tcp:{}", self.address))
    }
    fn lookup(&mut self, request: &LocalRequestIdentity) -> Result<EndpointStatus, ProtocolError> {
        match self.call(Command::Lookup(request.clone()))? {
            Value::Status(s) => Ok(s),
            _ => Err(ProtocolError::Poisoned),
        }
    }
    fn prepare(&mut self, proposal: &SignedProposal) -> Result<DurableReceipt, ProtocolError> {
        self.receipt(Command::Prepare(Box::new(proposal.clone())))
    }
    fn decide_primary(
        &mut self,
        proposal: &SignedProposal,
        prepared: &DurableReceipt,
    ) -> Result<DurableReceipt, ProtocolError> {
        self.receipt(Command::DecidePrimary(
            Box::new(proposal.clone()),
            Box::new(prepared.clone()),
        ))
    }
    fn decide_replica(
        &mut self,
        proposal: &SignedProposal,
        prepared: &DurableReceipt,
        primary: &DurableReceipt,
    ) -> Result<DurableReceipt, ProtocolError> {
        self.receipt(Command::DecideReplica(
            Box::new(proposal.clone()),
            Box::new(prepared.clone()),
            Box::new(primary.clone()),
        ))
    }
    fn finalize(&mut self, evidence: &DurabilityEvidence) -> Result<(), ProtocolError> {
        match self.call(Command::Finalize(Box::new(evidence.clone())))? {
            Value::Complete => Ok(()),
            _ => Err(ProtocolError::Poisoned),
        }
    }
}
