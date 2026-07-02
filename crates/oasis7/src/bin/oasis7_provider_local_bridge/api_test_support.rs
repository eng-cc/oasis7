use std::io::Write;

pub(super) fn write_http_response<W: Write>(
    stream: &mut W,
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
        502 => "Bad Gateway",
        503 => "Service Unavailable",
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
