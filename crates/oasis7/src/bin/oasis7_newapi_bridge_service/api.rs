use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

const HTTP_READ_TIMEOUT_SECS: u64 = 3;
const MAX_HTTP_HEADER_BYTES: usize = 32 * 1024;
const MAX_HTTP_BODY_BYTES: usize = 1024 * 1024;

#[derive(Debug)]
pub(super) struct HttpRequest {
    pub(super) method: String,
    pub(super) path: String,
    pub(super) body: Vec<u8>,
}

pub(super) fn read_http_request(stream: &mut TcpStream) -> Result<HttpRequest, String> {
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

    let mut content_length = 0usize;
    for line in lines {
        if line.is_empty() {
            continue;
        }
        let (name, value) = line
            .split_once(':')
            .ok_or_else(|| format!("invalid header line: {line}"))?;
        if name.trim().eq_ignore_ascii_case("content-length") {
            content_length = value
                .trim()
                .parse::<usize>()
                .map_err(|_| format!("invalid content-length: {}", value.trim()))?;
            if content_length > MAX_HTTP_BODY_BYTES {
                return Err("HTTP body is too large".to_string());
            }
        }
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

    Ok(HttpRequest { method, path, body })
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
        201 => "Created",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        409 => "Conflict",
        500 => "Internal Server Error",
        _ => "Error",
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
        .map_err(|err| format!("flush response failed: {err}"))
}

fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|window| window == b"\r\n\r\n")
}
