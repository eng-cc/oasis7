//! Bounded real TCP request reader; unused speculative connections carry no request.
use super::*;
#[derive(Default)]
pub(super) struct Outage {
    pub(super) active: AtomicBool,
    pub(super) observed: AtomicBool,
}
impl Outage {
    pub(super) fn reject(&self, stream: &TcpStream) -> bool {
        if !self.active.load(Ordering::SeqCst) {
            return false;
        }
        self.observed.store(true, Ordering::SeqCst);
        stream.shutdown(std::net::Shutdown::Both).unwrap();
        true
    }
}
pub(super) fn record_lookup_digest(bytes: &[u8], observed: &Arc<Mutex<Vec<String>>>) {
    let body = crate::feedback_submit_api::extract_http_json_body(bytes).unwrap();
    let request: oasis7::world_service::wire::AuthenticatedLookup =
        serde_json::from_slice(body).unwrap();
    let digest = correlation::key_digest(&request.request.key).unwrap();
    let mut observed = observed.lock().unwrap();
    assert!(
        observed.len() < 4096,
        "bounded conformance Lookup observation exhausted"
    );
    observed.push(digest);
}
pub(super) fn read_request(stream: &mut TcpStream) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        let n = match stream.read(&mut buffer) {
            Ok(0) if bytes.is_empty() => return Vec::new(),
            Ok(n) => n,
            Err(error)
                if bytes.is_empty()
                    && matches!(
                        error.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) =>
            {
                return Vec::new();
            }
            Err(error) => panic!("incomplete HTTP fixture request: {error}"),
        };
        assert!(n > 0, "truncated request");
        bytes.extend_from_slice(&buffer[..n]);
        assert!(bytes.len() <= 262_144 + 8192, "unbounded request");
        if let Some(end) = bytes.windows(4).position(|v| v == b"\r\n\r\n") {
            let header = std::str::from_utf8(&bytes[..end]).unwrap();
            let length = header
                .lines()
                .find_map(|line| {
                    let (key, value) = line.split_once(':')?;
                    key.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().unwrap())
                })
                .unwrap();
            if bytes.len() >= end + 4 + length {
                return bytes;
            }
        }
    }
}

/// Only a verified Describe response establishes readiness; no write is retried.
pub(super) fn wait_ready(client: &RemoteWorldServiceClient, worker: &thread::JoinHandle<()>) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        match client.describe() {
            Ok(_) => return,
            Err(error) => {
                eprintln!(
                    "fixture_describe_not_ready error={error:?} worker_finished={}",
                    worker.is_finished()
                );
                // The public client intentionally hides reqwest's source chain. A read-only
                // diagnostic repeats Describe on a separate transport, without reporting body/keys.
                let http = reqwest::blocking::Client::builder()
                    .timeout(Duration::from_secs(1))
                    .build()
                    .unwrap();
                let result = http
                    .post(format!("{}{}", client.config().endpoint, DESCRIBE_PATH))
                    .json(&DescribeWorldRequest {
                        contract_version: 1,
                        expected_world: client.config().expected_world.clone(),
                        trust_config_ref: client.config().trusted_service_public_key.clone(),
                    })
                    .send();
                match result {
                    Ok(response) => eprintln!(
                        "fixture_describe_transport_probe status={}",
                        response.status()
                    ),
                    Err(error) => {
                        use std::error::Error;
                        let mut chain = vec![error.to_string()];
                        let mut source = error.source();
                        while let Some(error) = source {
                            chain.push(error.to_string());
                            source = error.source();
                        }
                        eprintln!("fixture_describe_transport_chain={chain:?}");
                    }
                }
                assert!(
                    !worker.is_finished(),
                    "fixture listener worker ended before verified Describe"
                );
                assert!(
                    Instant::now() < deadline,
                    "fixture Describe readiness timeout: {error:?}"
                );
            }
        }
        thread::sleep(Duration::from_millis(10));
    }
}

/// Darwin accept inherits O_NONBLOCK from the listening socket. Record the real
/// descriptor flag, then use blocking request reads with a finite read timeout.
pub(super) fn configure_accepted_stream(stream: &TcpStream) {
    #[cfg(target_os = "macos")]
    {
        use std::os::fd::AsRawFd;
        unsafe extern "C" {
            fn fcntl(fd: i32, command: i32) -> i32;
        }
        // F_GETFL=3 and O_NONBLOCK=4 are Darwin descriptor constants.
        let flags = unsafe { fcntl(stream.as_raw_fd(), 3) };
        assert!(flags >= 0, "read accepted descriptor flags failed");
        eprintln!(
            "fixture_accepted_socket_flags={flags} inherited_nonblocking={}",
            flags & 4 != 0
        );
    }
    stream.set_nonblocking(false).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
}
