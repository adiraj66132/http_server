use http_server::config::Config;
use http_server::parser::{parse, Limits, Method, ParseError, ParseOutcome, Version};

fn default_limits() -> Limits {
    Limits::from(&Config::default())
}

#[test]
fn pr1_valid_get_request() {
    let req = b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
    let outcome = parse(req, &default_limits());
    match outcome {
        ParseOutcome::Complete { request, consumed } => {
            assert_eq!(request.method, Method::Get);
            assert_eq!(request.path, "/");
            assert_eq!(request.version, Version::Http11);
            assert_eq!(consumed, req.len());
        }
        _ => panic!("Expected Complete"),
    }
}

#[test]
fn pr1_valid_post_with_body() {
    let req = b"POST /echo HTTP/1.1\r\nHost: localhost\r\nContent-Length: 11\r\n\r\nhello world";
    let outcome = parse(req, &default_limits());
    match outcome {
        ParseOutcome::Complete { request, consumed } => {
            assert_eq!(request.method, Method::Post);
            assert_eq!(request.path, "/echo");
            assert_eq!(request.body, b"hello world");
            assert_eq!(consumed, req.len());
        }
        _ => panic!("Expected Complete"),
    }
}

#[test]
fn pr1_absolute_form_target_returns_400() {
    let req = b"GET http://example.com/ HTTP/1.1\r\nHost: localhost\r\n\r\n";
    let outcome = parse(req, &default_limits());
    assert_eq!(outcome, ParseOutcome::Error(ParseError::BadRequest));
}

#[test]
fn pr1_authority_form_target_returns_400() {
    let req = b"GET localhost:8080 HTTP/1.1\r\nHost: localhost\r\n\r\n";
    let outcome = parse(req, &default_limits());
    assert_eq!(outcome, ParseOutcome::Error(ParseError::BadRequest));
}

#[test]
fn pr2_bare_lf_returns_400() {
    let req = b"GET / HTTP/1.1\nHost: localhost\n\n";
    let outcome = parse(req, &default_limits());
    assert_eq!(outcome, ParseOutcome::Error(ParseError::BadRequest));
}

#[test]
fn pr3_version_http10_returns_505() {
    let req = b"GET / HTTP/1.0\r\nHost: localhost\r\n\r\n";
    let outcome = parse(req, &default_limits());
    assert_eq!(
        outcome,
        ParseOutcome::Error(ParseError::VersionNotSupported)
    );
}

#[test]
fn pr3_version_http20_returns_505() {
    let req = b"GET / HTTP/2.0\r\nHost: localhost\r\n\r\n";
    let outcome = parse(req, &default_limits());
    assert_eq!(
        outcome,
        ParseOutcome::Error(ParseError::VersionNotSupported)
    );
}

#[test]
fn pr4_missing_host_is_400() {
    let req = b"GET / HTTP/1.1\r\n\r\n";
    let outcome = parse(req, &default_limits());
    assert_eq!(outcome, ParseOutcome::Error(ParseError::HostRequired));
}

#[test]
fn pr4_duplicate_host_is_400() {
    let req = b"GET / HTTP/1.1\r\nHost: localhost\r\nHost: example.com\r\n\r\n";
    let outcome = parse(req, &default_limits());
    assert_eq!(outcome, ParseOutcome::Error(ParseError::BadRequest));
}

#[test]
fn pr5_obsolete_line_folding_returns_400() {
    let req = b"GET / HTTP/1.1\r\nHost: localhost\r\n X-Extra: value\r\n\r\n";
    let outcome = parse(req, &default_limits());
    assert_eq!(outcome, ParseOutcome::Error(ParseError::BadRequest));
}

#[test]
fn pr5_whitespace_before_colon_returns_400() {
    let req = b"GET / HTTP/1.1\r\nHost : localhost\r\n\r\n";
    let outcome = parse(req, &default_limits());
    assert_eq!(outcome, ParseOutcome::Error(ParseError::BadRequest));
}

#[test]
fn pr5_empty_header_name_returns_400() {
    let req = b"GET / HTTP/1.1\r\n: value\r\nHost: localhost\r\n\r\n";
    let outcome = parse(req, &default_limits());
    assert_eq!(outcome, ParseOutcome::Error(ParseError::BadRequest));
}

#[test]
fn pr5_invalid_header_name_returns_400() {
    let req = b"GET / HTTP/1.1\r\nHost@: localhost\r\n\r\n";
    let outcome = parse(req, &default_limits());
    assert_eq!(outcome, ParseOutcome::Error(ParseError::BadRequest));
}

#[test]
fn pr6_non_numeric_content_length_returns_400() {
    let req = b"POST /echo HTTP/1.1\r\nHost: localhost\r\nContent-Length: abc\r\n\r\n";
    let outcome = parse(req, &default_limits());
    assert_eq!(outcome, ParseOutcome::Error(ParseError::BadRequest));
}

#[test]
fn pr6_negative_content_length_returns_400() {
    let req = b"POST /echo HTTP/1.1\r\nHost: localhost\r\nContent-Length: -1\r\n\r\n";
    let outcome = parse(req, &default_limits());
    assert_eq!(outcome, ParseOutcome::Error(ParseError::BadRequest));
}

#[test]
fn pr6_multiple_content_length_returns_400() {
    let req = b"POST /echo HTTP/1.1\r\nHost: localhost\r\nContent-Length: 5\r\nContent-Length: 5\r\n\r\nhello";
    let outcome = parse(req, &default_limits());
    assert_eq!(outcome, ParseOutcome::Error(ParseError::BadRequest));
}

#[test]
fn pr6_content_length_with_transfer_encoding_returns_400() {
    let req = b"POST /echo HTTP/1.1\r\nHost: localhost\r\nContent-Length: 5\r\nTransfer-Encoding: chunked\r\n\r\nhello";
    let outcome = parse(req, &default_limits());
    assert_eq!(outcome, ParseOutcome::Error(ParseError::BadRequest));
}

#[test]
fn pr6_transfer_encoding_alone_returns_501() {
    let req = b"POST /echo HTTP/1.1\r\nHost: localhost\r\nTransfer-Encoding: chunked\r\n\r\n";
    let outcome = parse(req, &default_limits());
    assert_eq!(outcome, ParseOutcome::Error(ParseError::NotImplemented));
}

#[test]
fn pr6_post_no_content_length_returns_411() {
    let req = b"POST /echo HTTP/1.1\r\nHost: localhost\r\n\r\n";
    let outcome = parse(req, &default_limits());
    assert_eq!(outcome, ParseOutcome::Error(ParseError::LengthRequired));
}

#[test]
fn pr6_get_with_body_reads_and_discards() {
    let req = b"GET / HTTP/1.1\r\nHost: localhost\r\nContent-Length: 5\r\n\r\nhello";
    let outcome = parse(req, &default_limits());
    match outcome {
        ParseOutcome::Complete { request, consumed } => {
            assert_eq!(request.method, Method::Get);
            assert_eq!(request.body, b"hello");
            assert_eq!(consumed, req.len());
        }
        _ => panic!("Expected Complete"),
    }
}

#[test]
fn pr7_expect_100_continue_returns_417() {
    let req = b"GET / HTTP/1.1\r\nHost: localhost\r\nExpect: 100-continue\r\n\r\n";
    let outcome = parse(req, &default_limits());
    assert_eq!(outcome, ParseOutcome::Error(ParseError::ExpectationFailed));
}

#[test]
fn pr8_query_string_split_off() {
    let req = b"GET /health?foo=bar HTTP/1.1\r\nHost: localhost\r\n\r\n";
    let outcome = parse(req, &default_limits());
    match outcome {
        ParseOutcome::Complete { request, .. } => {
            assert_eq!(request.path, "/health");
            assert_eq!(request.query, Some("foo=bar".to_string()));
        }
        _ => panic!("Expected Complete"),
    }
}

#[test]
fn pr8_no_percent_decoding() {
    let req = b"GET /health%20test HTTP/1.1\r\nHost: localhost\r\n\r\n";
    let outcome = parse(req, &default_limits());
    match outcome {
        ParseOutcome::Complete { request, .. } => {
            assert_eq!(request.path, "/health%20test");
        }
        _ => panic!("Expected Complete"),
    }
}

#[test]
fn pr8_no_trailing_slash_normalization() {
    let req = b"GET /health/ HTTP/1.1\r\nHost: localhost\r\n\r\n";
    let outcome = parse(req, &default_limits());
    match outcome {
        ParseOutcome::Complete { request, .. } => {
            assert_eq!(request.path, "/health/");
        }
        _ => panic!("Expected Complete"),
    }
}

#[test]
fn pr9_pipelined_requests_consumed_correctly() {
    let req =
        b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\nGET /health HTTP/1.1\r\nHost: localhost\r\n\r\n";
    let outcome = parse(req, &default_limits());
    match outcome {
        ParseOutcome::Complete { request, consumed } => {
            assert_eq!(request.path, "/");
            assert_eq!(consumed, b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n".len());
        }
        _ => panic!("Expected Complete"),
    }
}

#[test]
fn ts2_incomplete_on_partial_input() {
    let req = b"GET / HTTP/1.1\r\nHost: localhost";
    let outcome = parse(req, &default_limits());
    assert_eq!(outcome, ParseOutcome::Incomplete);
}

#[test]
fn ts2_one_byte_at_a_time_equals_whole_buffer() {
    let full = b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
    let mut buf = Vec::new();
    let mut last_outcome = ParseOutcome::Incomplete;
    for byte in full {
        buf.push(*byte);
        let outcome = parse(&buf, &default_limits());
        if !matches!(outcome, ParseOutcome::Incomplete) {
            last_outcome = outcome;
            break;
        }
    }
    let whole_outcome = parse(full, &default_limits());
    assert_eq!(last_outcome, whole_outcome);
}

#[test]
fn ts2_two_pipelined_requests_give_correct_consumed() {
    let req1 = b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
    let req2 = b"GET /health HTTP/1.1\r\nHost: localhost\r\n\r\n";
    let combined = [req1.as_ref(), req2.as_ref()].concat();
    let outcome = parse(&combined, &default_limits());
    match outcome {
        ParseOutcome::Complete { request, consumed } => {
            assert_eq!(request.path, "/");
            assert_eq!(consumed, req1.len());
        }
        _ => panic!("Expected Complete"),
    }
}

#[test]
fn ts2_oversized_headers_returns_431() {
    let large_header = "X-Large: ".to_string() + &"x".repeat(10000);
    let req = format!(
        "GET / HTTP/1.1\r\nHost: localhost\r\n{}\r\n\r\n",
        large_header
    );
    let outcome = parse(req.as_bytes(), &default_limits());
    assert_eq!(
        outcome,
        ParseOutcome::Error(ParseError::HeaderFieldsTooLarge)
    );
}

#[test]
fn ts2_oversized_headers_431_while_incomplete() {
    let limits = Limits {
        max_header_bytes: 100,
        max_body_bytes: 1024,
    };
    let req = b"GET / HTTP/1.1\r\nHost: localhost\r\nX-Large: xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx";
    let outcome = parse(req, &limits);
    assert_eq!(
        outcome,
        ParseOutcome::Error(ParseError::HeaderFieldsTooLarge)
    );
}

#[test]
fn ts2_oversized_body_returns_413() {
    let limits = Limits {
        max_header_bytes: 8192,
        max_body_bytes: 10,
    };
    let req = b"POST /echo HTTP/1.1\r\nHost: localhost\r\nContent-Length: 20\r\n\r\n";
    let outcome = parse(req, &limits);
    assert_eq!(outcome, ParseOutcome::Error(ParseError::PayloadTooLarge));
}

#[test]
fn ts2_case_insensitive_header_names() {
    let req = b"GET / HTTP/1.1\r\nhOsT: localhost\r\n\r\n";
    let outcome = parse(req, &default_limits());
    match outcome {
        ParseOutcome::Complete { request, .. } => {
            assert!(request
                .headers
                .iter()
                .any(|(k, v)| k == "host" && v == "localhost"));
        }
        _ => panic!("Expected Complete"),
    }
}

#[test]
fn ts2_other_method_token_on_known_path_405_not_501() {
    let req = b"HEAD / HTTP/1.1\r\nHost: localhost\r\n\r\n";
    let outcome = parse(req, &default_limits());
    match outcome {
        ParseOutcome::Complete { request, .. } => {
            assert!(matches!(request.method, Method::Other(ref s) if s == "HEAD"));
        }
        _ => panic!("Expected Complete"),
    }
}
