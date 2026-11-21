# Traceability Table

## Requirements to Implementation Mapping

| Requirement ID | Description | Implementation Location | Test Coverage |
|----------------|-------------|------------------------|---------------|
| **PR-1** | Valid GET request parsing | `src/parser.rs:parse_request_line()` | `pr1_valid_get_request` |
| **PR-1** | Valid POST with body parsing | `src/parser.rs:parse()` | `pr1_valid_post_with_body` |
| **PR-1** | Absolute-form target returns 400 | `src/parser.rs:parse_request_line()` | `pr1_absolute_form_target_returns_400` |
| **PR-1** | Authority-form target returns 400 | `src/parser.rs:parse_request_line()` | `pr1_authority_form_target_returns_400` |
| **PR-2** | Bare LF returns 400 | `src/parser.rs:find_line_ending()` | `pr2_bare_lf_returns_400` |
| **PR-3** | HTTP/1.0 returns 505 | `src/parser.rs:Version::parse()` | `pr3_version_http10_returns_505` |
| **PR-3** | HTTP/2.0 returns 505 | `src/parser.rs:Version::parse()` | `pr3_version_http20_returns_505` |
| **PR-4** | Missing Host header returns 400 | `src/parser.rs:parse()` | `pr4_missing_host_is_400` |
| **PR-4** | Duplicate Host header returns 400 | `src/parser.rs:parse()` | `pr4_duplicate_host_is_400` |
| **PR-5** | Invalid header name returns 400 | `src/parser.rs:parse_headers()` | `pr5_invalid_header_name_returns_400` |
| **PR-5** | Empty header name returns 400 | `src/parser.rs:parse_headers()` | `pr5_empty_header_name_returns_400` |
| **PR-5** | Whitespace before colon returns 400 | `src/parser.rs:parse_headers()` | `pr5_whitespace_before_colon_returns_400` |
| **PR-5** | Obsolete line folding returns 400 | `src/parser.rs:find_line_ending()` | `pr5_obsolete_line_folding_returns_400` |
| **PR-6** | Content-Length with Transfer-Encoding returns 400 | `src/parser.rs:parse()` | `pr6_content_length_with_transfer_encoding_returns_400` |
| **PR-6** | Multiple Content-Length returns 400 | `src/parser.rs:parse()` | `pr6_multiple_content_length_returns_400` |
| **PR-6** | Negative Content-Length returns 400 | `src/parser.rs:parse()` | `pr6_negative_content_length_returns_400` |
| **PR-6** | Non-numeric Content-Length returns 400 | `src/parser.rs:parse()` | `pr6_non_numeric_content_length_returns_400` |
| **PR-6** | POST without Content-Length returns 411 | `src/parser.rs:parse()` | `pr6_post_no_content_length_returns_411` |
| **PR-6** | GET with body reads and discards | `src/parser.rs:parse()` | `pr6_get_with_body_reads_and_discards` |
| **PR-6** | Transfer-Encoding alone returns 501 | `src/parser.rs:parse()` | `pr6_transfer_encoding_alone_returns_501` |
| **PR-7** | Expect: 100-continue returns 417 | `src/parser.rs:parse()` | `pr7_expect_100_continue_returns_417` |
| **PR-8** | No percent decoding | `src/parser.rs:split_path_query()` | `pr8_no_percent_decoding` |
| **PR-8** | No trailing slash normalization | `src/parser.rs:split_path_query()` | `pr8_no_trailing_slash_normalization` |
| **PR-8** | Query string split off | `src/parser.rs:split_path_query()` | `pr8_query_string_split_off` |
| **PR-9** | Pipelined requests consumed correctly | `src/parser.rs:parse()` | `pr9_pipelined_requests_consumed_correctly` |

| Requirement ID | Description | Implementation Location | Test Coverage |
|----------------|-------------|------------------------|---------------|
| **FR-1** | GET / returns 200 | `src/router.rs:handle_root()` | `fr1_get_root_returns_200` |
| **FR-2** | GET /health returns 200 JSON | `src/router.rs:handle_health()` | `fr2_get_health_returns_200_json` |
| **FR-2** | POST /echo returns body | `src/router.rs:handle_echo()` | `fr2_post_echo_returns_body` |
| **FR-2** | POST /echo Content-Type handling | `src/router.rs:handle_echo()` | `fr2_post_echo_content_type` |
| **FR-3** | Method Not Allowed returns 405 with Allow | `src/router.rs:route()` | `fr3_method_not_allowed_returns_405_with_allow` |
| **FR-4** | Unknown path returns 404 | `src/router.rs:route()` | `fr4_unknown_path_returns_404` |
| **FR-4** | 404 over 405 precedence | `src/router.rs:route()` | `fr4_404_over_405_precedence` |
| **FR-5** | HEAD/OPTIONS/PUT/DELETE return 405 | `src/router.rs:route()` | `fr5_head_is_405`, `fr5_options_is_405`, `fr5_put_is_405`, `fr5_delete_is_405` |
| **FR-6** | Handler error returns 500 | `src/router.rs:route()` | `fr6_handler_error_returns_500` |

| Requirement ID | Description | Implementation Location | Test Coverage |
|----------------|-------------|------------------------|---------------|
| **TS-2** | Case insensitive header names | `src/parser.rs:parse_headers()` | `ts2_case_insensitive_header_names` |
| **TS-2** | Incomplete on partial input | `src/parser.rs:parse()` | `ts2_incomplete_on_partial_input` |
| **TS-2** | One byte at a time equals whole buffer | `src/parser.rs:parse()` | `ts2_one_byte_at_a_time_equals_whole_buffer` |
| **TS-2** | Other method on known path returns 405 not 501 | `src/router.rs:route()` | `ts2_other_method_token_on_known_path_405_not_501` |
| **TS-2** | Oversized body returns 413 | `src/parser.rs:parse()` | `ts2_oversized_body_returns_413` |
| **TS-2** | Oversized headers returns 431 | `src/parser.rs:parse()` / `parse_headers()` | `ts2_oversized_headers_returns_431` |
| **TS-2** | Oversized headers 431 while incomplete | `src/parser.rs:parse_headers()` | `ts2_oversized_headers_431_while_incomplete` |
| **TS-2** | Two pipelined requests give correct consumed | `src/parser.rs:parse()` | `ts2_two_pipelined_requests_give_correct_consumed` |

| Requirement ID | Description | Implementation Location | Test Coverage |
|----------------|-------------|------------------------|---------------|
| **TS-4a** | 20 concurrent clients keep-alive | `src/server.rs:run()` | `ts4a_20_concurrent_clients_keep_alive` |
| **TS-4b** | Garbage bytes then fresh connection | `src/server.rs:handle_connection()` | `ts4b_garbage_bytes_then_fresh_connection` |
| **TS-4c** | Slow client returns 408 | `src/server.rs:handle_connection()` | `ts4c_slow_client_returns_408` |
| **TS-4d** | Pipelined requests ordered responses | `src/server.rs:handle_connection()` | `ts4d_pipelined_requests_ordered_responses` |
| **TS-4e** | Connection cap max connections | `src/server.rs:run()` | `ts4e_connection_cap_max_connections_2` |
| **TS-4f** | Graceful shutdown in-flight completes | `src/server.rs:run()` | `ts4f_graceful_shutdown_in_flight_completes` |

| Requirement ID | Description | Implementation Location | Test Coverage |
|----------------|-------------|------------------------|---------------|
| **RS-1** | Response has status line, Content-Length, Content-Type | `src/response.rs:serialize()` | `rs1_response_has_status_line_content_length_content_type` |
| **RS-2** | Error response has Connection: close | `src/router.rs:Response::not_found()` etc. | `rs2_error_response_has_connection_close` |
| **RS-3** | Client Connection: close -> server responds close | `src/server.rs:handle_connection()` | `rs3_client_connection_close_server_responds_close` |
| **RS-3** | Success no Connection header persistent | `src/server.rs:handle_connection()` | `rs3_success_no_connection_header_persistent` |

| Requirement ID | Description | Implementation Location | Test Coverage |
|----------------|-------------|------------------------|---------------|
| **PR-6 (Integration)** | GET with body reads and discards | `src/parser.rs:parse()` + `src/server.rs` | `pr6_get_with_body_reads_and_discards_integration` |
| **PR-9 (Integration)** | Pipelining sequential processing | `src/server.rs:handle_connection()` | `pr9_pipelining_sequential_processing` |

## Test Summary

- **Unit Tests**: 44 passing (33 parser + 11 router)
- **Integration Tests**: 12 passing
- **Total**: 76 tests passing
- **Formatter**: `cargo fmt --check` passes
- **Linter**: `cargo clippy --all-targets -- -D warnings` passes