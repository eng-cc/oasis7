use super::*;
#[test]
fn forwarding_headers_only_count_when_loopback_proxy_is_explicitly_trusted() {
    let headers =
        "POST / HTTP/1.1\r\nX-Real-IP: 203.0.113.8\r\nX-Forwarded-For: 198.51.100.4\r\n\r\n";
    assert_eq!(
        identity_source("127.0.0.1".parse().unwrap(), true, headers).unwrap(),
        "203.0.113.8"
    );
    assert_eq!(
        identity_source("127.0.0.1".parse().unwrap(), false, headers).unwrap(),
        "127.0.0.1"
    );
    assert_eq!(
        identity_source("198.51.100.6".parse().unwrap(), true, headers).unwrap(),
        "198.51.100.6"
    );
    assert!(
        identity_source(
            "127.0.0.1".parse().unwrap(),
            true,
            "POST / HTTP/1.1\r\n\r\n"
        )
        .is_err()
    );
    assert!(
        identity_source(
            "127.0.0.1".parse().unwrap(),
            true,
            "POST / HTTP/1.1\r\nX-Real-IP: 203.0.113.8\r\nX-Real-IP: 198.51.100.4\r\n\r\n"
        )
        .is_err()
    );
    assert_eq!(
        identity_source("::ffff:203.0.113.8".parse().unwrap(), false, headers).unwrap(),
        "203.0.113.8"
    );
}
#[test]
fn ambiguous_and_oversized_request_framing_is_rejected() {
    assert!(
        parse_content_length("POST / HTTP/1.1\r\nContent-Length: 2\r\nContent-Length: 2").is_err()
    );
    assert!(parse_content_length("POST / HTTP/1.1\r\nTransfer-Encoding: chunked").is_err());
    assert!(parse_content_length("POST / HTTP/1.1\r\nContent-Length: 65537").is_err());
    assert_eq!(
        parse_content_length("POST / HTTP/1.1\r\nContent-Length: 42").unwrap(),
        42
    );
}
#[test]
fn connection_budget_rejects_queue_growth_and_releases_permits() {
    let budget = ConnectionBudget::new();
    let permits = (0..32)
        .map(|_| budget.try_acquire().unwrap())
        .collect::<Vec<_>>();
    assert!(budget.try_acquire().is_none());
    drop(permits);
    assert!(budget.try_acquire().is_some());
}
