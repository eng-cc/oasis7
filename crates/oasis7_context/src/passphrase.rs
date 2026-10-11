use crate::error::{ContextError, Result};
use std::io::{IsTerminal, Read, Write};
use zeroize::Zeroizing;

pub fn read(stdin: bool, confirm: bool) -> Result<Zeroizing<Vec<u8>>> {
    if stdin {
        if std::io::stdin().is_terminal() {
            return Err(failed("passphrase stdin must be a pipe"));
        }
        let mut bytes = Zeroizing::new(Vec::new());
        std::io::stdin().take(1027).read_to_end(&mut bytes)?;
        trim_newline(&mut bytes);
        validate(&bytes)?;
        return Ok(bytes);
    }
    let first = hidden("Snapshot passphrase: ")?;
    validate(&first)?;
    if confirm {
        let second = hidden("Confirm passphrase: ")?;
        if *first != *second {
            return Err(failed("passphrase confirmation differs"));
        }
    }
    Ok(first)
}
fn trim_newline(bytes: &mut Vec<u8>) {
    if bytes.last() == Some(&b'\n') {
        bytes.pop();
        if bytes.last() == Some(&b'\r') {
            bytes.pop();
        }
    }
}
fn validate(bytes: &[u8]) -> Result<()> {
    if !(12..=1024).contains(&bytes.len()) {
        Err(failed("passphrase must contain 12 to 1024 bytes"))
    } else {
        Ok(())
    }
}
fn failed(message: &str) -> ContextError {
    ContextError::new("CRYPTO_ERROR", message)
}

#[cfg(unix)]
#[allow(unsafe_code)] // The TTY descriptor and termios restoration are confined to this function.
fn hidden(prompt: &str) -> Result<Zeroizing<Vec<u8>>> {
    use std::os::fd::AsRawFd;
    let mut tty = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")
        .map_err(|_| failed("interactive passphrase requires a terminal"))?;
    let fd = tty.as_raw_fd();
    // SAFETY: fd is a live TTY descriptor and termios is initialized by tcgetattr.
    let original = unsafe {
        let mut value = std::mem::zeroed::<libc::termios>();
        if libc::tcgetattr(fd, &mut value) != 0 {
            return Err(failed("terminal configuration failed"));
        }
        value
    };
    struct Restore(i32, libc::termios);
    impl Drop for Restore {
        fn drop(&mut self) {
            // SAFETY: the guard drops before the owning TTY file.
            unsafe {
                libc::tcsetattr(self.0, libc::TCSANOW, &self.1);
            }
        }
    }
    let mut no_echo = original;
    no_echo.c_lflag &= !libc::ECHO;
    // SAFETY: fd refers to the opened terminal and no_echo is a valid termios value.
    if unsafe { libc::tcsetattr(fd, libc::TCSANOW, &no_echo) } != 0 {
        return Err(failed("terminal configuration failed"));
    }
    let _restore = Restore(fd, original);
    tty.write_all(prompt.as_bytes())?;
    tty.flush()?;
    let mut bytes = Zeroizing::new(Vec::new());
    loop {
        let mut byte = [0];
        if tty.read(&mut byte)? == 0 {
            return Err(failed("passphrase input ended"));
        }
        if byte[0] == b'\n' {
            break;
        }
        if bytes.len() >= 1024 {
            // Consume the canonical input line before restoring echo and returning to the shell.
            while tty.read(&mut byte)? != 0 && byte[0] != b'\n' {}
            tty.write_all(b"\n")?;
            return Err(failed("passphrase exceeds limit"));
        }
        bytes.push(byte[0]);
    }
    tty.write_all(b"\n")?;
    Ok(bytes)
}
#[cfg(not(unix))]
fn hidden(_: &str) -> Result<Zeroizing<Vec<u8>>> {
    Err(ContextError::new(
        "PLATFORM_UNSUPPORTED",
        "context supports macOS and Linux",
    ))
}
