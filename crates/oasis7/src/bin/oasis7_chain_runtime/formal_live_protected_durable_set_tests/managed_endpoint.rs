use super::*;
use std::os::unix::fs::OpenOptionsExt;
use std::{
    io::{BufRead, BufReader},
    process::{Child, Command, Stdio},
    sync::mpsc::{self, Receiver},
};
pub(crate) struct ManagedEndpointChild {
    child: Child,
    config: PathBuf,
    stderr: PathBuf,
    lines: Receiver<String>,
    reader: Option<thread::JoinHandle<()>>,
}
impl ManagedEndpointChild {
    pub(crate) fn id(&self) -> u32 {
        self.child.id()
    }
    pub(crate) fn start(binary: &std::ffi::OsStr, config: &Path) -> Result<Self, String> {
        let stderr = config.with_extension(format!(
            "{}.startup.stderr",
            crate::node_lifecycle::local_io::random_id()?
        ));
        let stderr_file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(&stderr)
            .map_err(|e| e.to_string())?;
        let mut child = Command::new(binary)
            .arg("--config")
            .arg(config)
            .stdout(Stdio::piped())
            .stderr(Stdio::from(stderr_file))
            .spawn()
            .map_err(|error| format!("start healthy protected endpoint: {error}"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or("healthy endpoint stdout unavailable")?;
        let (sender, lines) = mpsc::channel();
        let reader = thread::Builder::new()
            .name("healthy-endpoint-readiness".into())
            .spawn(move || {
                for line in BufReader::new(stdout).lines() {
                    match line {
                        Ok(line) => {
                            if sender.send(line).is_err() {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
            })
            .map_err(|error| error.to_string())?;
        Ok(Self {
            child,
            config: config.to_path_buf(),
            stderr,
            lines,
            reader: Some(reader),
        })
    }
    pub(crate) fn wait_management(
        &mut self,
        expected: std::net::SocketAddr,
        timeout: Duration,
    ) -> Result<(), String> {
        self.wait_address("formal_endpoint_awaiting_management=", expected, timeout)
    }
    pub(crate) fn wait_ordinary(
        &mut self,
        expected: std::net::SocketAddr,
        timeout: Duration,
    ) -> Result<(), String> {
        self.wait_address("formal_endpoint_listening=", expected, timeout)
    }
    fn failure(&mut self, prefix: &str, seen: &[String]) -> String {
        use std::io::Read;
        let status = self.child.try_wait();
        let mut stderr = String::new();
        if let Ok(file) = std::fs::File::open(&self.stderr) {
            let _ = file.take(16 * 1024).read_to_string(&mut stderr);
        }
        format!(
            "endpoint missing {prefix}; pid={} status={status:?} config={} stderr_path={} stderr={stderr:?} output={seen:?}",
            self.child.id(),
            self.config.display(),
            self.stderr.display()
        )
    }
    fn wait_address(
        &mut self,
        prefix: &str,
        expected: std::net::SocketAddr,
        timeout: Duration,
    ) -> Result<(), String> {
        let deadline = Instant::now() + timeout;
        let mut seen = Vec::new();
        loop {
            let remaining = deadline
                .checked_duration_since(Instant::now())
                .filter(|value| !value.is_zero())
                .ok_or_else(|| self.failure(prefix, &seen))?;
            match self.lines.recv_timeout(remaining) {
                Ok(line) if line.starts_with(prefix) => {
                    let address = line[prefix.len()..]
                        .parse::<std::net::SocketAddr>()
                        .map_err(|error| format!("parse endpoint readiness {line:?}: {error}"))?;
                    if address != expected {
                        return Err(format!(
                            "endpoint readiness address {address} differs from expected {expected}"
                        ));
                    }
                    return Ok(());
                }
                Ok(line) => seen.push(line),
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    return Err(self.failure(prefix, &seen));
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(self.failure(prefix, &seen));
                }
            }
        }
    }
}
impl Drop for ManagedEndpointChild {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }
}
