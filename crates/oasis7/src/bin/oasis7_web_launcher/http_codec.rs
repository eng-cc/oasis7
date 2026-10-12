use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use serde::Serialize;

const HTTP_READ_TIMEOUT_SECS: u64 = 3;
const HTTP_WRITE_TIMEOUT_SECS: u64 = 3;
const MAX_HTTP_HEADER_BYTES: usize = 32 * 1024;
const MAX_HTTP_BODY_BYTES: usize = 1024 * 1024;

#[derive(Debug)]
pub(super) struct HttpRequest {
    pub(super) method: String,
    pub(super) path: String,
    pub(super) headers: Vec<(String, String)>,
    pub(super) body: Vec<u8>,
}

pub(super) fn read_http_request(stream: &mut TcpStream) -> Result<HttpRequest, String> {
    // Accepted sockets can inherit the listener's nonblocking mode on macOS.
    // Each connection has its own worker, so use bounded blocking I/O instead
    // of truncating large static responses when write_all encounters WouldBlock.
    stream
        .set_nonblocking(false)
        .map_err(|err| format!("set connection blocking mode failed: {err}"))?;
    stream
        .set_write_timeout(Some(Duration::from_secs(HTTP_WRITE_TIMEOUT_SECS)))
        .map_err(|err| format!("set write timeout failed: {err}"))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(HTTP_READ_TIMEOUT_SECS)))
        .map_err(|err| format!("set read timeout failed: {err}"))?;

    let mut buffer = Vec::with_capacity(1024);
    let header_end = loop {
        if buffer.len() > MAX_HTTP_HEADER_BYTES {
            return Err("HTTP header is too large".to_string());
        }
        let mut chunk = [0_u8; 1024];
        let bytes = stream
            .read(&mut chunk)
            .map_err(|err| format!("read request failed: {err}"))?;
        if bytes == 0 {
            return Err("empty request".to_string());
        }
        buffer.extend_from_slice(&chunk[..bytes]);
        if let Some(end) = find_header_end(buffer.as_slice()) {
            break end;
        }
    };

    let header_bytes = &buffer[..header_end];
    let header_text = String::from_utf8_lossy(header_bytes);
    let mut lines = header_text.split("\r\n");
    let request_line = lines
        .next()
        .ok_or_else(|| "missing request line".to_string())?;
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts
        .next()
        .ok_or_else(|| "missing request method".to_string())?
        .to_ascii_uppercase();
    let path = request_parts
        .next()
        .ok_or_else(|| "missing request target".to_string())?
        .to_string();

    let mut headers = Vec::new();
    let mut content_length = 0usize;
    for line in lines {
        if line.is_empty() {
            continue;
        }
        let (name, value) = line
            .split_once(':')
            .ok_or_else(|| format!("invalid header line: {line}"))?;
        let name = name.trim().to_ascii_lowercase();
        let value = value.trim().to_string();
        if name == "content-length" {
            content_length = value
                .parse::<usize>()
                .map_err(|_| format!("invalid content-length: {value}"))?;
            if content_length > MAX_HTTP_BODY_BYTES {
                return Err("HTTP body is too large".to_string());
            }
        }
        headers.push((name, value));
    }

    let mut body = buffer[(header_end + 4)..].to_vec();
    while body.len() < content_length {
        let remaining = content_length - body.len();
        let mut chunk = vec![0_u8; remaining.min(4096)];
        let bytes = stream
            .read(chunk.as_mut_slice())
            .map_err(|err| format!("read request body failed: {err}"))?;
        if bytes == 0 {
            return Err("unexpected EOF while reading request body".to_string());
        }
        body.extend_from_slice(&chunk[..bytes]);
    }
    body.truncate(content_length);

    Ok(HttpRequest {
        method,
        path,
        headers,
        body,
    })
}

pub(super) fn write_json_response<T: Serialize>(
    stream: &mut TcpStream,
    status_code: u16,
    payload: &T,
) -> Result<(), String> {
    let body =
        serde_json::to_vec(payload).map_err(|err| format!("serialize JSON failed: {err}"))?;
    write_http_response(
        stream,
        status_code,
        "application/json; charset=utf-8",
        body.as_slice(),
        false,
    )
}

pub(super) fn write_http_response(
    stream: &mut TcpStream,
    status_code: u16,
    content_type: &str,
    body: &[u8],
    head_only: bool,
) -> Result<(), String> {
    let status_text = match status_code {
        200 => "OK",
        204 => "No Content",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        _ => "Internal Server Error",
    };
    let headers = format!(
        "HTTP/1.1 {status_code} {status_text}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream
        .write_all(headers.as_bytes())
        .map_err(|err| format!("write response header failed: {err}"))?;
    if !head_only {
        stream
            .write_all(body)
            .map_err(|err| format!("write response body failed: {err}"))?;
    }
    stream
        .flush()
        .map_err(|err| format!("flush response failed: {err}"))?;
    Ok(())
}

fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|window| window == b"\r\n\r\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn nonblocking_connection_delivers_complete_large_response() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let address = listener.local_addr().expect("address");
        let worker = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            // Reproduce the accepted-socket mode inherited from a nonblocking
            // listener on macOS; explicit mode also makes this portable.
            stream.set_nonblocking(true).expect("nonblocking");
            read_http_request(&mut stream)?;
            write_http_response(
                &mut stream,
                200,
                "application/wasm",
                &vec![0x42; 32 * 1024 * 1024],
                false,
            )
        });
        let mut client = TcpStream::connect(address).expect("connect");
        client
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("read deadline");
        client
            .write_all(b"GET /launcher.wasm HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .expect("request");
        std::thread::sleep(Duration::from_millis(100));
        let mut response = Vec::new();
        client.read_to_end(&mut response).expect("read response");
        worker.join().expect("worker").expect("complete response");
        let boundary = find_header_end(&response).expect("HTTP headers") + 4;
        assert_eq!(response.len() - boundary, 32 * 1024 * 1024);
        assert!(response[boundary..].iter().all(|byte| *byte == 0x42));
    }
}
