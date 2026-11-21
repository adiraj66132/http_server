use crate::router::Response;

pub fn serialize(response: &Response) -> Vec<u8> {
    let mut buf = Vec::new();
    let status_text = status_text(response.status);
    buf.extend_from_slice(format!("HTTP/1.1 {} {}\r\n", response.status, status_text).as_bytes());

    for (name, value) in &response.headers {
        buf.extend_from_slice(format!("{}: {}\r\n", name, value).as_bytes());
    }

    buf.extend_from_slice(b"\r\n");
    buf.extend_from_slice(&response.body);
    buf
}

fn status_text(code: u16) -> &'static str {
    match code {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        408 => "Request Timeout",
        411 => "Length Required",
        413 => "Payload Too Large",
        417 => "Expectation Failed",
        431 => "Request Header Fields Too Large",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        505 => "HTTP Version Not Supported",
        _ => "Unknown",
    }
}
