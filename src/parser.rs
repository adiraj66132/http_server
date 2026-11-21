use crate::config::Config;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub method: Method,
    pub path: String,
    pub query: Option<String>,
    pub version: Version,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
    Other(String),
}

impl Method {
    pub fn parse(s: &str) -> Self {
        match s {
            "GET" => Method::Get,
            "POST" => Method::Post,
            other => Method::Other(other.to_string()),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Method::Get => "GET",
            Method::Post => "POST",
            Method::Other(s) => s,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Version {
    Http11,
    Other(u8, u8),
}

impl Version {
    pub fn parse(s: &str) -> Option<Self> {
        if s == "HTTP/1.1" {
            Some(Version::Http11)
        } else if let Some(rest) = s.strip_prefix("HTTP/") {
            let parts: Vec<&str> = rest.split('.').collect();
            if parts.len() == 2 {
                let major = parts[0].parse().ok()?;
                let minor = parts[1].parse().ok()?;
                Some(Version::Other(major, minor))
            } else {
                None
            }
        } else {
            None
        }
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ParseError {
    #[error("bad request")]
    BadRequest,
    #[error("version not supported")]
    VersionNotSupported,
    #[error("host header required")]
    HostRequired,
    #[error("method not allowed")]
    MethodNotAllowed,
    #[error("not found")]
    NotFound,
    #[error("request timeout")]
    RequestTimeout,
    #[error("length required")]
    LengthRequired,
    #[error("payload too large")]
    PayloadTooLarge,
    #[error("expectation failed")]
    ExpectationFailed,
    #[error("header fields too large")]
    HeaderFieldsTooLarge,
    #[error("internal server error")]
    InternalServerError,
    #[error("not implemented")]
    NotImplemented,
}

impl ParseError {
    pub fn status_code(&self) -> u16 {
        match self {
            ParseError::BadRequest => 400,
            ParseError::VersionNotSupported => 505,
            ParseError::HostRequired => 400,
            ParseError::MethodNotAllowed => 405,
            ParseError::NotFound => 404,
            ParseError::RequestTimeout => 408,
            ParseError::LengthRequired => 411,
            ParseError::PayloadTooLarge => 413,
            ParseError::ExpectationFailed => 417,
            ParseError::HeaderFieldsTooLarge => 431,
            ParseError::InternalServerError => 500,
            ParseError::NotImplemented => 501,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Limits {
    pub max_header_bytes: usize,
    pub max_body_bytes: usize,
}

impl From<&Config> for Limits {
    fn from(config: &Config) -> Self {
        Self {
            max_header_bytes: config.max_header_bytes,
            max_body_bytes: config.max_body_bytes,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseOutcome {
    Complete { request: Request, consumed: usize },
    Incomplete,
    Error(ParseError),
}

pub fn parse(buf: &[u8], limits: &Limits) -> ParseOutcome {
    let mut cursor = 0;

    let request_line = match parse_request_line(buf, &mut cursor) {
        ParseResult::Ok(rl) => rl,
        ParseResult::Incomplete => return ParseOutcome::Incomplete,
        ParseResult::Error(e) => return ParseOutcome::Error(e),
    };

    if cursor > limits.max_header_bytes {
        return ParseOutcome::Error(ParseError::HeaderFieldsTooLarge);
    }

    let headers = match parse_headers(buf, &mut cursor, limits) {
        ParseResult::Ok(h) => h,
        ParseResult::Incomplete => return ParseOutcome::Incomplete,
        ParseResult::Error(e) => return ParseOutcome::Error(e),
    };

    if cursor > limits.max_header_bytes {
        return ParseOutcome::Error(ParseError::HeaderFieldsTooLarge);
    }

    let total_header_consumed = cursor;
    let (method, path, query, version) = request_line;

    if version != Version::Http11 {
        return ParseOutcome::Error(ParseError::VersionNotSupported);
    }

    let host_header = headers.iter().find(|(k, _)| k.eq_ignore_ascii_case("host"));
    if host_header.is_none() {
        return ParseOutcome::Error(ParseError::HostRequired);
    }
    let host_count = headers
        .iter()
        .filter(|(k, _)| k.eq_ignore_ascii_case("host"))
        .count();
    if host_count > 1 {
        return ParseOutcome::Error(ParseError::BadRequest);
    }

    let content_length = headers
        .iter()
        .filter(|(k, _)| k.eq_ignore_ascii_case("content-length"))
        .count();
    if content_length > 1 {
        return ParseOutcome::Error(ParseError::BadRequest);
    }

    let has_transfer_encoding = headers
        .iter()
        .any(|(k, _)| k.eq_ignore_ascii_case("transfer-encoding"));

    if content_length > 0 && has_transfer_encoding {
        return ParseOutcome::Error(ParseError::BadRequest);
    }

    let content_length_val = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-length"))
        .map(|(_, v)| v.as_str());

    let expects_100_continue = headers
        .iter()
        .any(|(k, v)| k.eq_ignore_ascii_case("expect") && v.eq_ignore_ascii_case("100-continue"));
    if expects_100_continue {
        return ParseOutcome::Error(ParseError::ExpectationFailed);
    }

    let body_len = match content_length_val {
        Some(cl_str) => match cl_str.parse::<usize>() {
            Ok(len) => {
                if len > limits.max_body_bytes {
                    return ParseOutcome::Error(ParseError::PayloadTooLarge);
                }
                Some(len)
            }
            Err(_) => return ParseOutcome::Error(ParseError::BadRequest),
        },
        None => {
            if matches!(method, Method::Post) && !has_transfer_encoding {
                return ParseOutcome::Error(ParseError::LengthRequired);
            }
            if has_transfer_encoding {
                return ParseOutcome::Error(ParseError::NotImplemented);
            }
            None
        }
    };

    let remaining = &buf[total_header_consumed..];
    let body = if let Some(expected_len) = body_len {
        if remaining.len() < expected_len {
            return ParseOutcome::Incomplete;
        }
        remaining[..expected_len].to_vec()
    } else {
        Vec::new()
    };

    let consumed = total_header_consumed + body.len();

    let request = Request {
        method,
        path,
        query,
        version,
        headers,
        body,
    };

    ParseOutcome::Complete { request, consumed }
}

enum ParseResult<T> {
    Ok(T),
    Incomplete,
    Error(ParseError),
}

fn parse_request_line(
    buf: &[u8],
    cursor: &mut usize,
) -> ParseResult<(Method, String, Option<String>, Version)> {
    let start = *cursor;
    let (end, is_crlf) = match find_line_ending(buf, start) {
        Some((e, crlf)) => (e, crlf),
        None => return ParseResult::Incomplete,
    };
    if !is_crlf {
        return ParseResult::Error(ParseError::BadRequest);
    }
    let line = &buf[start..end];
    *cursor = end + 2;

    let parts: Vec<&[u8]> = line.split(|b| *b == b' ').collect();
    if parts.len() != 3 {
        return ParseResult::Error(ParseError::BadRequest);
    }

    let method = match std::str::from_utf8(parts[0]) {
        Ok(s) => Method::parse(s),
        Err(_) => return ParseResult::Error(ParseError::BadRequest),
    };

    let target = match std::str::from_utf8(parts[1]) {
        Ok(s) => s,
        Err(_) => return ParseResult::Error(ParseError::BadRequest),
    };
    if target.starts_with("http://") || target.starts_with("https://") || !target.starts_with('/') {
        return ParseResult::Error(ParseError::BadRequest);
    }

    let (path, query) = split_path_query(target);

    let version_str = match std::str::from_utf8(parts[2]) {
        Ok(s) => s,
        Err(_) => return ParseResult::Error(ParseError::BadRequest),
    };
    let version = match Version::parse(version_str) {
        Some(v) => v,
        None => return ParseResult::Error(ParseError::BadRequest),
    };

    ParseResult::Ok((
        method,
        path.to_string(),
        query.map(|s| s.to_string()),
        version,
    ))
}

fn find_line_ending(buf: &[u8], start: usize) -> Option<(usize, bool)> {
    for i in start..buf.len().saturating_sub(1) {
        if buf[i] == b'\r' && buf[i + 1] == b'\n' {
            return Some((i, true));
        }
        if buf[i] == b'\n' {
            return Some((i, false));
        }
    }
    if buf.len() > start && buf[buf.len() - 1] == b'\n' {
        return Some((buf.len() - 1, false));
    }
    None
}

fn split_path_query(target: &str) -> (&str, Option<&str>) {
    if let Some(idx) = target.find('?') {
        (&target[..idx], Some(&target[idx + 1..]))
    } else {
        (target, None)
    }
}

fn parse_headers(
    buf: &[u8],
    cursor: &mut usize,
    limits: &Limits,
) -> ParseResult<Vec<(String, String)>> {
    let mut headers = Vec::new();

    loop {
        if *cursor >= buf.len() {
            return ParseResult::Incomplete;
        }

        if *cursor + 1 < buf.len() && &buf[*cursor..*cursor + 2] == b"\r\n" {
            *cursor += 2;
            break;
        }

        if *cursor > limits.max_header_bytes {
            return ParseResult::Error(ParseError::HeaderFieldsTooLarge);
        }

        let (line_end, is_crlf) = match find_line_ending(buf, *cursor) {
            Some((e, crlf)) => (e, crlf),
            None => {
                if buf.len() > limits.max_header_bytes {
                    return ParseResult::Error(ParseError::HeaderFieldsTooLarge);
                }
                return ParseResult::Incomplete;
            }
        };
        if !is_crlf {
            return ParseResult::Error(ParseError::BadRequest);
        }
        let line = &buf[*cursor..line_end];
        *cursor = line_end + 2;

        if line.is_empty() {
            continue;
        }

        if line[0].is_ascii_whitespace() {
            return ParseResult::Error(ParseError::BadRequest);
        }

        let colon_pos = match line.iter().position(|b| *b == b':') {
            Some(p) => p,
            None => return ParseResult::Error(ParseError::BadRequest),
        };
        if colon_pos == 0 {
            return ParseResult::Error(ParseError::BadRequest);
        }

        let name = &line[..colon_pos];
        if name.iter().any(|b| b.is_ascii_whitespace()) {
            return ParseResult::Error(ParseError::BadRequest);
        }

        if !name.iter().all(|b| {
            b.is_ascii_graphic()
                && *b != b':'
                && *b != b'('
                && *b != b')'
                && *b != b'<'
                && *b != b'>'
                && *b != b'@'
                && *b != b','
                && *b != b';'
                && *b != b':'
                && *b != b'\\'
                && *b != b'\"'
                && *b != b'/'
                && *b != b'['
                && *b != b']'
                && *b != b'?'
                && *b != b'='
                && *b != b'{'
                && *b != b'}'
        }) {
            return ParseResult::Error(ParseError::BadRequest);
        }

        let value_start = colon_pos + 1;
        let value = if value_start < line.len() {
            &line[value_start..]
        } else {
            b""
        };

        let name_str = match std::str::from_utf8(name) {
            Ok(s) => s,
            Err(_) => return ParseResult::Error(ParseError::BadRequest),
        };
        let value_str = match std::str::from_utf8(trim_ascii_start(value)) {
            Ok(s) => s,
            Err(_) => return ParseResult::Error(ParseError::BadRequest),
        };

        headers.push((name_str.to_lowercase(), value_str.to_string()));
    }

    ParseResult::Ok(headers)
}

fn trim_ascii_start(bytes: &[u8]) -> &[u8] {
    let mut start = 0;
    while start < bytes.len() && bytes[start].is_ascii_whitespace() {
        start += 1;
    }
    &bytes[start..]
}
