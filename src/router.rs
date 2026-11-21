use crate::parser::{Method, Request};
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum RouterError {
    #[error("handler error")]
    HandlerError,
}

pub type HandlerFn = fn(&Request) -> Result<Response, RouterError>;

#[derive(Debug, Clone)]
pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Response {
    pub fn ok_text(body: impl Into<Vec<u8>>) -> Self {
        let body = body.into();
        Self {
            status: 200,
            headers: vec![
                (
                    "content-type".to_string(),
                    "text/plain; charset=utf-8".to_string(),
                ),
                ("content-length".to_string(), body.len().to_string()),
            ],
            body,
        }
    }

    pub fn ok_json(body: impl Into<Vec<u8>>) -> Self {
        let body = body.into();
        Self {
            status: 200,
            headers: vec![
                ("content-type".to_string(), "application/json".to_string()),
                ("content-length".to_string(), body.len().to_string()),
            ],
            body,
        }
    }

    pub fn ok_echo(content_type: String, body: Vec<u8>) -> Self {
        Self {
            status: 200,
            headers: vec![
                ("content-type".to_string(), content_type),
                ("content-length".to_string(), body.len().to_string()),
            ],
            body,
        }
    }

    pub fn not_found() -> Self {
        let body = b"Not Found".to_vec();
        Self {
            status: 404,
            headers: vec![
                (
                    "content-type".to_string(),
                    "text/plain; charset=utf-8".to_string(),
                ),
                ("content-length".to_string(), body.len().to_string()),
                ("connection".to_string(), "close".to_string()),
            ],
            body,
        }
    }

    pub fn method_not_allowed(allowed: &[Method]) -> Self {
        let allow = allowed
            .iter()
            .map(|m| m.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        let body = b"Method Not Allowed".to_vec();
        Self {
            status: 405,
            headers: vec![
                (
                    "content-type".to_string(),
                    "text/plain; charset=utf-8".to_string(),
                ),
                ("content-length".to_string(), body.len().to_string()),
                ("allow".to_string(), allow),
                ("connection".to_string(), "close".to_string()),
            ],
            body,
        }
    }

    pub fn bad_request() -> Self {
        let body = b"Bad Request".to_vec();
        Self {
            status: 400,
            headers: vec![
                (
                    "content-type".to_string(),
                    "text/plain; charset=utf-8".to_string(),
                ),
                ("content-length".to_string(), body.len().to_string()),
                ("connection".to_string(), "close".to_string()),
            ],
            body,
        }
    }

    pub fn version_not_supported() -> Self {
        let body = b"HTTP Version Not Supported".to_vec();
        Self {
            status: 505,
            headers: vec![
                (
                    "content-type".to_string(),
                    "text/plain; charset=utf-8".to_string(),
                ),
                ("content-length".to_string(), body.len().to_string()),
                ("connection".to_string(), "close".to_string()),
            ],
            body,
        }
    }

    pub fn length_required() -> Self {
        let body = b"Length Required".to_vec();
        Self {
            status: 411,
            headers: vec![
                (
                    "content-type".to_string(),
                    "text/plain; charset=utf-8".to_string(),
                ),
                ("content-length".to_string(), body.len().to_string()),
                ("connection".to_string(), "close".to_string()),
            ],
            body,
        }
    }

    pub fn payload_too_large() -> Self {
        let body = b"Payload Too Large".to_vec();
        Self {
            status: 413,
            headers: vec![
                (
                    "content-type".to_string(),
                    "text/plain; charset=utf-8".to_string(),
                ),
                ("content-length".to_string(), body.len().to_string()),
                ("connection".to_string(), "close".to_string()),
            ],
            body,
        }
    }

    pub fn expectation_failed() -> Self {
        let body = b"Expectation Failed".to_vec();
        Self {
            status: 417,
            headers: vec![
                (
                    "content-type".to_string(),
                    "text/plain; charset=utf-8".to_string(),
                ),
                ("content-length".to_string(), body.len().to_string()),
                ("connection".to_string(), "close".to_string()),
            ],
            body,
        }
    }

    pub fn header_fields_too_large() -> Self {
        let body = b"Request Header Fields Too Large".to_vec();
        Self {
            status: 431,
            headers: vec![
                (
                    "content-type".to_string(),
                    "text/plain; charset=utf-8".to_string(),
                ),
                ("content-length".to_string(), body.len().to_string()),
                ("connection".to_string(), "close".to_string()),
            ],
            body,
        }
    }

    pub fn internal_server_error() -> Self {
        let body = b"Internal Server Error".to_vec();
        Self {
            status: 500,
            headers: vec![
                (
                    "content-type".to_string(),
                    "text/plain; charset=utf-8".to_string(),
                ),
                ("content-length".to_string(), body.len().to_string()),
                ("connection".to_string(), "close".to_string()),
            ],
            body,
        }
    }

    pub fn not_implemented() -> Self {
        let body = b"Not Implemented".to_vec();
        Self {
            status: 501,
            headers: vec![
                (
                    "content-type".to_string(),
                    "text/plain; charset=utf-8".to_string(),
                ),
                ("content-length".to_string(), body.len().to_string()),
                ("connection".to_string(), "close".to_string()),
            ],
            body,
        }
    }

    pub fn request_timeout() -> Self {
        let body = b"Request Timeout".to_vec();
        Self {
            status: 408,
            headers: vec![
                (
                    "content-type".to_string(),
                    "text/plain; charset=utf-8".to_string(),
                ),
                ("content-length".to_string(), body.len().to_string()),
                ("connection".to_string(), "close".to_string()),
            ],
            body,
        }
    }

    pub fn with_connection_close(mut self, close: bool) -> Self {
        if close {
            self.headers
                .push(("connection".to_string(), "close".to_string()));
        } else {
            self.headers
                .retain(|(k, _)| !k.eq_ignore_ascii_case("connection"));
        }
        self
    }
}

pub fn route(request: &Request) -> Result<Response, RouterError> {
    let allowed_methods = match request.path.as_str() {
        "/" => &[Method::Get][..],
        "/health" => &[Method::Get][..],
        "/echo" => &[Method::Post][..],
        "/error" => &[Method::Get][..],
        _ => return Ok(Response::not_found()),
    };

    let method_allowed = allowed_methods
        .iter()
        .any(|m| std::mem::discriminant(m) == std::mem::discriminant(&request.method));
    if !method_allowed {
        return Ok(Response::method_not_allowed(allowed_methods));
    }

    match (&request.method, request.path.as_str()) {
        (Method::Get, "/") => Ok(handle_root()),
        (Method::Get, "/health") => Ok(handle_health()),
        (Method::Post, "/echo") => Ok(handle_echo(request)),
        (Method::Get, "/error") => Err(RouterError::HandlerError),
        _ => Ok(Response::internal_server_error()),
    }
}

fn handle_root() -> Response {
    Response::ok_text("Hello, World!")
}

fn handle_health() -> Response {
    Response::ok_json(r#"{"status":"ok"}"#)
}

fn handle_echo(request: &Request) -> Response {
    let content_type = request
        .headers
        .iter()
        .find(|(k, _)| k == "content-type")
        .map(|(_, v)| v.clone())
        .unwrap_or_else(|| "application/octet-stream".to_string());
    Response::ok_echo(content_type, request.body.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{Method, Request, Version};

    fn make_request(method: Method, path: &str, body: Vec<u8>) -> Request {
        Request {
            method,
            path: path.to_string(),
            query: None,
            version: Version::Http11,
            headers: vec![],
            body,
        }
    }

    #[test]
    fn fr1_get_root_returns_200() {
        let req = make_request(Method::Get, "/", vec![]);
        let resp = route(&req).unwrap();
        assert_eq!(resp.status, 200);
        assert!(resp.body.starts_with(b"Hello"));
    }

    #[test]
    fn fr2_get_health_returns_200_json() {
        let req = make_request(Method::Get, "/health", vec![]);
        let resp = route(&req).unwrap();
        assert_eq!(resp.status, 200);
        assert!(resp
            .headers
            .iter()
            .any(|(k, v)| k == "content-type" && v == "application/json"));
        assert_eq!(resp.body, br#"{"status":"ok"}"#);
    }

    #[test]
    fn fr2_post_echo_returns_body() {
        let req = make_request(Method::Post, "/echo", b"test body".to_vec());
        let resp = route(&req).unwrap();
        assert_eq!(resp.status, 200);
        assert_eq!(resp.body, b"test body");
    }

    #[test]
    fn fr2_post_echo_content_type() {
        let mut req = make_request(Method::Post, "/echo", b"data".to_vec());
        req.headers
            .push(("content-type".to_string(), "application/json".to_string()));
        let resp = route(&req).unwrap();
        assert_eq!(
            resp.headers
                .iter()
                .find(|(k, _)| k == "content-type")
                .unwrap()
                .1,
            "application/json"
        );
    }

    #[test]
    fn fr3_method_not_allowed_returns_405_with_allow() {
        let req = make_request(Method::Post, "/", vec![]);
        let resp = route(&req).unwrap();
        assert_eq!(resp.status, 405);
        assert!(resp.headers.iter().any(|(k, _)| k == "allow"));
    }

    #[test]
    fn fr4_unknown_path_returns_404() {
        let req = make_request(Method::Get, "/unknown", vec![]);
        let resp = route(&req).unwrap();
        assert_eq!(resp.status, 404);
    }

    #[test]
    fn fr4_404_over_405_precedence() {
        let req = make_request(Method::Post, "/unknown", vec![]);
        let resp = route(&req).unwrap();
        assert_eq!(resp.status, 404);
    }

    #[test]
    fn fr5_head_options_put_delete_are_405() {
        for method in [
            Method::Other("HEAD".into()),
            Method::Other("OPTIONS".into()),
            Method::Other("PUT".into()),
            Method::Other("DELETE".into()),
        ] {
            let req = make_request(method, "/", vec![]);
            let resp = route(&req).unwrap();
            assert_eq!(resp.status, 405, "method {:?} should be 405", req.method);
        }
    }

    #[test]
    fn fr6_handler_error_returns_500() {
        let req = make_request(Method::Get, "/error", vec![]);
        let resp = route(&req).unwrap_err();
        assert_eq!(resp, RouterError::HandlerError);
    }

    #[test]
    fn ts4_query_string_ignored() {
        let mut req = make_request(Method::Get, "/health", vec![]);
        req.query = Some("foo=bar".to_string());
        let resp = route(&req).unwrap();
        assert_eq!(resp.status, 200);
    }

    #[test]
    fn ts4_no_trailing_slash_normalization() {
        let req = make_request(Method::Get, "/health/", vec![]);
        let resp = route(&req).unwrap();
        assert_eq!(resp.status, 404);
    }
}
