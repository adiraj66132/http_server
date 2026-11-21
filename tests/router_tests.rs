use http_server::parser::{Method, Request, Version};
use http_server::router::{route, RouterError};

fn make_request(method: Method, path: &str, body: Vec<u8>) -> Request {
    let (path, query) = if let Some(idx) = path.find('?') {
        (&path[..idx], Some(path[idx + 1..].to_string()))
    } else {
        (path, None)
    };
    Request {
        method,
        path: path.to_string(),
        query,
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
fn fr2_post_echo_default_content_type() {
    let req = make_request(Method::Post, "/echo", b"data".to_vec());
    let resp = route(&req).unwrap();
    assert_eq!(
        resp.headers
            .iter()
            .find(|(k, _)| k == "content-type")
            .unwrap()
            .1,
        "application/octet-stream"
    );
}

#[test]
fn fr3_method_not_allowed_returns_405_with_allow() {
    let req = make_request(Method::Post, "/", vec![]);
    let resp = route(&req).unwrap();
    assert_eq!(resp.status, 405);
    let allow_header = resp.headers.iter().find(|(k, _)| k == "allow").unwrap();
    assert!(allow_header.1.contains("GET"));
}

#[test]
fn fr3_method_not_allowed_health_returns_405_with_allow() {
    let req = make_request(Method::Post, "/health", vec![]);
    let resp = route(&req).unwrap();
    assert_eq!(resp.status, 405);
    let allow_header = resp.headers.iter().find(|(k, _)| k == "allow").unwrap();
    assert!(allow_header.1.contains("GET"));
}

#[test]
fn fr3_method_not_allowed_echo_returns_405_with_allow() {
    let req = make_request(Method::Get, "/echo", vec![]);
    let resp = route(&req).unwrap();
    assert_eq!(resp.status, 405);
    let allow_header = resp.headers.iter().find(|(k, _)| k == "allow").unwrap();
    assert!(allow_header.1.contains("POST"));
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
fn fr5_head_is_405() {
    let req = make_request(Method::Other("HEAD".into()), "/", vec![]);
    let resp = route(&req).unwrap();
    assert_eq!(resp.status, 405);
}

#[test]
fn fr5_options_is_405() {
    let req = make_request(Method::Other("OPTIONS".into()), "/", vec![]);
    let resp = route(&req).unwrap();
    assert_eq!(resp.status, 405);
}

#[test]
fn fr5_put_is_405() {
    let req = make_request(Method::Other("PUT".into()), "/", vec![]);
    let resp = route(&req).unwrap();
    assert_eq!(resp.status, 405);
}

#[test]
fn fr5_delete_is_405() {
    let req = make_request(Method::Other("DELETE".into()), "/", vec![]);
    let resp = route(&req).unwrap();
    assert_eq!(resp.status, 405);
}

#[test]
fn fr6_handler_error_returns_500() {
    let req = make_request(Method::Get, "/error", vec![]);
    let err = route(&req).unwrap_err();
    assert_eq!(err, RouterError::HandlerError);
}

#[test]
fn ts4_query_string_ignored_in_routing() {
    let req = make_request(Method::Get, "/health?foo=bar", vec![]);
    let resp = route(&req).unwrap();
    assert_eq!(resp.status, 200);
}

#[test]
fn ts4_no_trailing_slash_normalization() {
    let req = make_request(Method::Get, "/health/", vec![]);
    let resp = route(&req).unwrap();
    assert_eq!(resp.status, 404);
}

#[test]
fn ts4_response_has_content_length() {
    let req = make_request(Method::Get, "/", vec![]);
    let resp = route(&req).unwrap();
    assert!(resp.headers.iter().any(|(k, _)| k == "content-length"));
}

#[test]
fn ts4_error_response_has_connection_close() {
    let req = make_request(Method::Get, "/unknown", vec![]);
    let resp = route(&req).unwrap();
    assert!(resp
        .headers
        .iter()
        .any(|(k, v)| k == "connection" && v == "close"));
}

#[test]
fn ts4_success_response_no_connection_header_by_default() {
    let req = make_request(Method::Get, "/", vec![]);
    let resp = route(&req).unwrap();
    assert!(!resp.headers.iter().any(|(k, _)| k == "connection"));
}
