//! Bounded retries apply only to two public read-only snapshot race categories.
use super::*;
use crate::WorldServiceClientError as E;
use oasis7_client_api::world_service::task_factory_quote::{
    TASK_QUOTE_BASE_DRIFT, TASK_QUOTE_DELIVERY_DRIFT,
};
use std::time::Instant;
fn drift(error: &E) -> bool {
    matches!(error, E::Rejected { status:409, message } if [TASK_QUOTE_BASE_DRIFT,TASK_QUOTE_DELIVERY_DRIFT].contains(&message.as_str()))
}
fn retry<C, T>(
    mut capture: impl FnMut() -> Result<C, String>,
    identity: impl Fn(&C) -> Value,
    mut quote: impl FnMut(&C) -> Result<T, E>,
    mut guard: impl FnMut() -> Result<(), String>,
    window: Duration,
) -> Result<(C, T), String> {
    let deadline = Instant::now() + window;
    guard()?;
    let mut captured = capture()?;
    let original_identity = identity(&captured);
    for attempt in 0..3 {
        guard()?;
        if Instant::now() >= deadline {
            break;
        }
        match quote(&captured) {
            Ok(quoted) => return Ok((captured, quoted)),
            Err(error) if drift(&error) => {
                if attempt == 2 || Instant::now() >= deadline {
                    break;
                }
                guard()?;
                captured = capture()?;
                if identity(&captured) != original_identity {
                    return Err("quote context identity changed; no action created".into());
                }
            }
            Err(error) => {
                return Err(format!(
                    "quote {}; no replacement action created",
                    failure_tag(&error)
                ));
            }
        }
    }
    Err("quote_conflict_retry_exhausted; original context retained; no action created".into())
}
impl Client {
    pub(super) fn ensure_quote_has_no_write(&self, operation: &str) -> Result<(), String> {
        for kind in [ClientStateKind::Action, ClientStateKind::Outcome] {
            if self.cached_intent(kind, operation)?.is_some() {
                return Err("original write exists; query original result".into());
            }
        }
        Ok(())
    }
    pub(super) fn fresh_quote<T>(
        &self,
        operation: &str,
        quote: impl FnMut(&CapturedTaskContextV1) -> Result<T, E>,
    ) -> Result<(CapturedTaskContextV1, T), String> {
        retry(
            || self.captured(operation),
            |c| serde_json::to_value(&c.prepared).expect("serializable prepared context"),
            quote,
            || self.ensure_quote_has_no_write(operation),
            Duration::from_secs(8),
        )
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    fn conflict(code: &str) -> E {
        E::Rejected {
            status: 409,
            message: code.into(),
        }
    }
    #[test]
    fn snapshot_drift_refreshes_same_identity_before_success() {
        for code in [TASK_QUOTE_BASE_DRIFT, TASK_QUOTE_DELIVERY_DRIFT] {
            let captures = Cell::new(0);
            let attempts = Cell::new(0);
            let got = retry(
                || {
                    captures.set(captures.get() + 1);
                    Ok(("same-original", captures.get()))
                },
                |c| json!(c.0),
                |c| {
                    attempts.set(attempts.get() + 1);
                    if c.1 == 1 {
                        Err(conflict(code))
                    } else {
                        Ok(c.1)
                    }
                },
                || Ok(()),
                Duration::from_secs(8),
            )
            .unwrap();
            assert_eq!(got.1, 2);
            assert_eq!(captures.get(), 2);
            assert_eq!(attempts.get(), 2);
        }
    }
    #[test]
    fn generic_conflicts_permission_expiry_and_assurance_never_retry() {
        for error in [
            conflict("private_goal_conflict"),
            conflict("context_expired"),
            E::Rejected {
                status: 403,
                message: TASK_QUOTE_BASE_DRIFT.into(),
            },
            E::Assurance("signature".into()),
        ] {
            let attempts = Cell::new(0);
            let mut error = Some(error);
            assert!(
                retry(
                    || Ok(()),
                    |_| Value::Null,
                    |_| {
                        attempts.set(attempts.get() + 1);
                        Err::<(), _>(error.take().unwrap())
                    },
                    || Ok(()),
                    Duration::from_secs(8)
                )
                .is_err()
            );
            assert_eq!(attempts.get(), 1);
        }
    }
    #[test]
    fn drift_ceiling_deadline_identity_change_and_cached_write_fail_closed() {
        let attempts = Cell::new(0);
        assert!(
            retry(
                || Ok(()),
                |_| Value::Null,
                |_| {
                    attempts.set(attempts.get() + 1);
                    Err::<(), _>(conflict(TASK_QUOTE_BASE_DRIFT))
                },
                || Ok(()),
                Duration::from_secs(8)
            )
            .unwrap_err()
            .contains("quote_conflict_retry_exhausted")
        );
        assert_eq!(attempts.get(), 3);
        let attempts = Cell::new(0);
        assert!(
            retry(
                || Ok(()),
                |_| Value::Null,
                |_| {
                    attempts.set(attempts.get() + 1);
                    Ok(())
                },
                || Ok(()),
                Duration::ZERO
            )
            .is_err()
        );
        assert_eq!(attempts.get(), 0);
        let captures = Cell::new(0);
        assert!(
            retry(
                || {
                    captures.set(captures.get() + 1);
                    Ok(captures.get())
                },
                |c| json!(c),
                |_| Err::<(), _>(conflict(TASK_QUOTE_BASE_DRIFT)),
                || Ok(()),
                Duration::from_secs(8)
            )
            .unwrap_err()
            .contains("identity changed")
        );
        let attempts = Cell::new(0);
        assert!(
            retry(
                || Ok(()),
                |_| Value::Null,
                |_| {
                    attempts.set(attempts.get() + 1);
                    Ok(())
                },
                || Err("cached write".into()),
                Duration::from_secs(8)
            )
            .is_err()
        );
        assert_eq!(attempts.get(), 0);
    }
    #[test]
    fn private_machine_category_is_strict_bounded_and_quote_path_only() {
        use oasis7_client_api::world_service::task_factory_quote::TASK_FACTORY_QUOTE_PATH;
        let classify =
            |path, status, body: &[u8]| crate::quote_error_category(path, status, &mut &body[..]);
        assert_eq!(
            classify(
                TASK_FACTORY_QUOTE_PATH,
                409,
                br#"{"error":"task_quote_base_drift"}"#
            ),
            Some(TASK_QUOTE_BASE_DRIFT)
        );
        for body in [
            br#"{"error":"private_goal_conflict"}"#.as_slice(),
            br#"{"error":"task_quote_base_drift","private":"hidden"}"#,
            br#"{"error":"task_quote_base_drift","error":"task_quote_base_drift"}"#,
        ] {
            assert_eq!(classify(TASK_FACTORY_QUOTE_PATH, 409, body), None);
        }
        assert_eq!(
            classify("/other", 409, br#"{"error":"task_quote_base_drift"}"#),
            None
        );
        assert_eq!(
            classify(
                TASK_FACTORY_QUOTE_PATH,
                403,
                br#"{"error":"task_quote_base_drift"}"#
            ),
            None
        );
        assert_eq!(classify(TASK_FACTORY_QUOTE_PATH, 409, &[b' '; 257]), None);
    }
    #[test]
    fn actual_http_quote_error_categories_drive_only_bounded_read_refresh() {
        use oasis7_client_api::world_service::task_factory_quote::TASK_FACTORY_QUOTE_PATH;
        use std::{
            io::{Read, Write},
            net::TcpListener,
            thread,
        };
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let transport = crate::WorldServiceTransport::new(crate::tests::config(format!(
            "http://{}",
            listener.local_addr().unwrap()
        )))
        .unwrap();
        let server = thread::spawn(move || {
            for (expected, body) in [
                (1, br#"{"error":"task_quote_base_drift"}"#.as_slice()),
                (2, br#"{"error":"task_quote_delivery_drift"}"#),
                (3, br#"{"version":3}"#),
            ] {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut bytes = Vec::new();
                let mut buf = [0; 1024];
                loop {
                    let n = stream.read(&mut buf).unwrap();
                    assert!(n > 0);
                    bytes.extend_from_slice(&buf[..n]);
                    if let Some(end) = bytes.windows(4).position(|v| v == b"\r\n\r\n") {
                        let head = String::from_utf8_lossy(&bytes[..end]);
                        let length: usize = head
                            .lines()
                            .find_map(|l| {
                                l.to_ascii_lowercase()
                                    .strip_prefix("content-length:")
                                    .map(|v| v.trim().parse().unwrap())
                            })
                            .unwrap();
                        if bytes.len() >= end + 4 + length {
                            let request: Value =
                                serde_json::from_slice(&bytes[end + 4..end + 4 + length]).unwrap();
                            assert_eq!(request, json!({"context":"original","binding":expected}));
                            break;
                        }
                    }
                }
                write!(stream,"HTTP/1.1 {} Result\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",if expected<3 {409}else{200},body.len()).unwrap();
                stream.write_all(body).unwrap();
            }
        });
        let captures = Cell::new(0);
        let (context, bytes) = retry(
            || {
                captures.set(captures.get() + 1);
                Ok(("original", captures.get()))
            },
            |c| json!(c.0),
            |c| {
                transport.private_response_bytes(
                    TASK_FACTORY_QUOTE_PATH,
                    &json!({"context":c.0,"binding":c.1}),
                    1024,
                )
            },
            || Ok(()),
            Duration::from_secs(8),
        )
        .unwrap();
        assert_eq!(context, ("original", 3));
        assert_eq!(captures.get(), 3);
        assert_eq!(
            serde_json::from_slice::<Value>(&bytes).unwrap(),
            json!({"version":3})
        );
        server.join().unwrap();
    }
}
