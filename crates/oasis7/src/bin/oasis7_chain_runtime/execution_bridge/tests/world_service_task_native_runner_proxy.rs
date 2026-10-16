//! Test-only delivery race. Forward exact signed bytes; never replace a request.
use super::*;
use std::net::{Shutdown, TcpStream};

pub(super) struct OriginalDeliveryRace {
    endpoint: String,
    stop: Arc<AtomicBool>,
    delayed: Arc<AtomicBool>,
    worker: Option<std::thread::JoinHandle<()>>,
}
impl OriginalDeliveryRace {
    pub(super) fn start(
        upstream: &str,
        root: PathBuf,
        world: WorldIdentity,
        deadline: Instant,
    ) -> Self {
        let upstream = upstream.strip_prefix("http://").unwrap().to_owned();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let stop = Arc::new(AtomicBool::new(false));
        let delayed = Arc::new(AtomicBool::new(false));
        let stopping = stop.clone();
        let observed = delayed.clone();
        let worker = std::thread::spawn(move || {
            while !stopping.load(Ordering::SeqCst) {
                let (mut incoming, _) = match listener.accept() {
                    Ok(connection) => connection,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(5));
                        continue;
                    }
                    Err(_) => panic!("conformance proxy accept failed"),
                };
                incoming.set_nonblocking(false).unwrap();
                incoming
                    .set_read_timeout(Some(Duration::from_secs(15)))
                    .unwrap();
                let mut bytes = Vec::new();
                while !bytes.ends_with(b"\r\n\r\n") {
                    let mut byte = [0];
                    if incoming.read_exact(&mut byte).is_err() {
                        break;
                    }
                    bytes.push(byte[0]);
                    assert!(bytes.len() <= 65536, "proxy header limit");
                }
                if !bytes.ends_with(b"\r\n\r\n") {
                    continue;
                }
                let headers = std::str::from_utf8(&bytes).unwrap();
                let path = headers.split_whitespace().nth(1).unwrap().to_owned();
                let length = headers
                    .split("\r\n")
                    .filter_map(|line| line.split_once(':'))
                    .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                    .map(|(_, value)| value.trim().parse::<usize>().unwrap())
                    .unwrap_or(0);
                assert!(length <= 256 * 1024, "proxy request limit");
                let mut body = vec![0; length];
                incoming.read_exact(&mut body).unwrap();
                let first_transfer = path == SUBMIT_PATH && !observed.load(Ordering::SeqCst)
                    && serde_json::from_slice::<SubmitIntentRequest<WorldServicePayloadV1>>(&body)
                    .is_ok_and(|request| matches!(request.signed_payload,
                        WorldServicePayloadV1::Cognition(signed) if matches!(signed.request.action, Action::TransferMaterial {..})));
                bytes.extend(body);
                if first_transfer {
                    let pin = || {
                        super::super::super::super::super::world_service_read::pin(
                            &root.join("records"),
                            &root.join("store"),
                            &world,
                            None,
                        )
                        .unwrap()
                    };
                    let before = pin().commit;
                    loop {
                        if pin().commit != before {
                            break;
                        }
                        assert!(Instant::now() < deadline, "proxy normal tick deadline");
                        assert!(
                            !stopping.load(Ordering::SeqCst),
                            "proxy interrupted before delivery"
                        );
                        std::thread::sleep(Duration::from_millis(20));
                    }
                    observed.store(true, Ordering::SeqCst);
                }
                let mut outgoing = TcpStream::connect(&upstream).unwrap();
                outgoing
                    .set_read_timeout(Some(Duration::from_secs(15)))
                    .unwrap();
                outgoing.write_all(&bytes).unwrap();
                outgoing.shutdown(Shutdown::Write).unwrap();
                let mut response = Vec::new();
                outgoing
                    .take(4 * 1024 * 1024 + 1)
                    .read_to_end(&mut response)
                    .unwrap();
                assert!(response.len() <= 4 * 1024 * 1024, "proxy response limit");
                let _ = incoming.write_all(&response);
            }
        });
        Self {
            endpoint,
            stop,
            delayed,
            worker: Some(worker),
        }
    }
    pub(super) fn endpoint(&self) -> &str {
        &self.endpoint
    }
    pub(super) fn delayed_original(&self) -> bool {
        self.delayed.load(Ordering::SeqCst)
    }
}
impl Drop for OriginalDeliveryRace {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(worker) = self.worker.take() {
            // Preserve an existing assertion during unwinding; the worker never
            // owns a model or a Runtime execution authority.
            let result = worker.join();
            if !std::thread::panicking() {
                result.unwrap();
            }
        }
    }
}
