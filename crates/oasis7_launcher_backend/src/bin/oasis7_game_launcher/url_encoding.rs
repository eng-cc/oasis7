pub(super) fn encoded_query_pair(key: &str, value: &str) -> String {
    format!("{key}={}", encode_query_value(value))
}

fn encode_query_value(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(byte as char);
        } else {
            encoded.push('%');
            encoded.push(hex_upper(byte >> 4));
            encoded.push(hex_upper(byte & 0x0f));
        }
    }
    encoded
}

fn hex_upper(nibble: u8) -> char {
    match nibble {
        0..=9 => (b'0' + nibble) as char,
        10..=15 => (b'A' + (nibble - 10)) as char,
        _ => '0',
    }
}
