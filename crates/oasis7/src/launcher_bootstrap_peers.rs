use multiaddr::Multiaddr;

const MULTIADDR_EXAMPLE: &str = "/ip4/127.0.0.1/tcp/4100/p2p/<peer-id>";

pub fn parse_chain_replication_bootstrap_peer(raw: &str) -> Result<String, String> {
    let token = raw.trim();
    if token.is_empty() {
        return Err("chain replication bootstrap peer cannot be empty".to_string());
    }
    token.parse::<Multiaddr>().map_err(|err| {
        format!(
            "chain replication bootstrap peers must use libp2p multiaddr values like {MULTIADDR_EXAMPLE}; invalid `{token}`: {err}"
        )
    })?;
    Ok(token.to_string())
}

pub fn parse_chain_replication_bootstrap_peers(raw: &str) -> Result<Vec<String>, String> {
    let mut peers = Vec::new();
    for token in raw.split([',', ';', ' ', '\n', '\r', '\t']) {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }
        peers.push(parse_chain_replication_bootstrap_peer(token)?);
    }
    Ok(peers)
}

#[cfg(test)]
mod tests {
    use super::{parse_chain_replication_bootstrap_peer, parse_chain_replication_bootstrap_peers};

    #[test]
    fn parse_chain_replication_bootstrap_peer_accepts_valid_multiaddr() {
        let peer = parse_chain_replication_bootstrap_peer("/ip4/127.0.0.1/tcp/4100")
            .expect("valid multiaddr");
        assert_eq!(peer, "/ip4/127.0.0.1/tcp/4100".to_string());
    }

    #[test]
    fn parse_chain_replication_bootstrap_peer_rejects_non_multiaddr() {
        let err = parse_chain_replication_bootstrap_peer("/not-a-multiaddr")
            .expect_err("invalid multiaddr should fail");
        assert!(err.contains("multiaddr"));
        assert!(err.contains("/not-a-multiaddr"));
    }

    #[test]
    fn parse_chain_replication_bootstrap_peers_accepts_common_delimiters() {
        let peers = parse_chain_replication_bootstrap_peers(
            "/ip4/127.0.0.1/tcp/4100,\n/dns4/bootstrap.example/tcp/4101",
        )
        .expect("should parse peers");
        assert_eq!(
            peers,
            vec![
                "/ip4/127.0.0.1/tcp/4100".to_string(),
                "/dns4/bootstrap.example/tcp/4101".to_string(),
            ]
        );
    }
}
