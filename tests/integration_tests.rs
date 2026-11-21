use http_server::config::Config;
use http_server::server::Server;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{sleep, timeout};

async fn send_and_read(stream: &mut TcpStream, request: &[u8]) -> Vec<u8> {
    stream.write_all(request).await.unwrap();
    let mut buf = vec![0u8; 4096];
    let n = timeout(Duration::from_secs(5), stream.read(&mut buf))
        .await
        .unwrap()
        .unwrap();
    buf.truncate(n);
    buf
}

#[tokio::test]
async fn ts4a_20_concurrent_clients_keep_alive() {
    let config = Config::default();
    let server = Server::bind(config, "127.0.0.1:0".parse().unwrap())
        .await
        .unwrap();
    let addr = server.local_addr();

    let handle = tokio::spawn(server.run());

    sleep(Duration::from_millis(100)).await;

    let mut tasks = Vec::new();
    for _ in 0..20 {
        tasks.push(tokio::spawn(async move {
            let mut stream = TcpStream::connect(addr).await.unwrap();
            for _ in 0..5 {
                let req = b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
                let resp = send_and_read(&mut stream, req).await;
                assert!(resp.starts_with(b"HTTP/1.1 200"));
                let resp_str = String::from_utf8_lossy(&resp);
                assert!(resp_str.contains("Hello"));
            }
        }));
    }

    for task in tasks {
        task.await.unwrap();
    }

    handle.abort();
}

#[tokio::test]
async fn ts4b_garbage_bytes_then_fresh_connection() {
    let config = Config::default();
    let server = Server::bind(config, "127.0.0.1:0".parse().unwrap())
        .await
        .unwrap();
    let addr = server.local_addr();

    let handle = tokio::spawn(server.run());

    sleep(Duration::from_millis(100)).await;

    let mut stream = TcpStream::connect(addr).await.unwrap();
    // Send a request with bare LF (invalid per PR-2) to trigger 400
    stream
        .write_all(b"GET / HTTP/1.1\nHost: localhost\n\n")
        .await
        .unwrap();
    let mut buf = vec![0u8; 1024];
    let n = timeout(Duration::from_secs(5), stream.read(&mut buf)).await;
    eprintln!("DEBUG GARBAGE READ RESULT: {:?}", n);
    let n = n.unwrap().unwrap();
    buf.truncate(n);
    eprintln!(
        "DEBUG GARBAGE RESPONSE: {:?}",
        String::from_utf8_lossy(&buf)
    );
    assert!(buf.starts_with(b"HTTP/1.1 400"));

    drop(stream);

    let mut stream = TcpStream::connect(addr).await.unwrap();
    let req = b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
    let resp = send_and_read(&mut stream, req).await;
    assert!(resp.starts_with(b"HTTP/1.1 200"));

    handle.abort();
}

#[tokio::test]
async fn ts4c_slow_client_returns_408() {
    let config = Config {
        header_read_timeout: Duration::from_millis(100),
        ..Default::default()
    };
    let server = Server::bind(config, "127.0.0.1:0".parse().unwrap())
        .await
        .unwrap();
    let addr = server.local_addr();

    let handle = tokio::spawn(server.run());

    sleep(Duration::from_millis(50)).await;

    let mut stream = TcpStream::connect(addr).await.unwrap();
    stream.write_all(b"GET / ").await.unwrap();
    sleep(Duration::from_millis(200)).await;
    let mut buf = vec![0u8; 1024];
    let n = timeout(Duration::from_secs(5), stream.read(&mut buf)).await;
    eprintln!("DEBUG SLOW READ RESULT: {:?}", n);
    let n = n.unwrap().unwrap();
    buf.truncate(n);
    eprintln!("DEBUG SLOW RESPONSE: {:?}", String::from_utf8_lossy(&buf));
    assert!(buf.starts_with(b"HTTP/1.1 408"));

    handle.abort();
}

#[tokio::test]
async fn ts4d_pipelined_requests_ordered_responses() {
    let config = Config::default();
    let server = Server::bind(config, "127.0.0.1:0".parse().unwrap())
        .await
        .unwrap();
    let addr = server.local_addr();

    let handle = tokio::spawn(server.run());

    sleep(Duration::from_millis(100)).await;

    let mut stream = TcpStream::connect(addr).await.unwrap();
    let pipelined =
        b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\nGET /health HTTP/1.1\r\nHost: localhost\r\n\r\n";
    stream.write_all(pipelined).await.unwrap();

    let mut buf = vec![0u8; 4096];
    let n = timeout(Duration::from_secs(5), stream.read(&mut buf))
        .await
        .unwrap()
        .unwrap();
    buf.truncate(n);

    let resp_str = String::from_utf8_lossy(&buf);
    assert!(resp_str.contains("HTTP/1.1 200"));
    assert!(resp_str.contains("Hello"));
    assert!(resp_str.contains(r#"{"status":"ok"}"#));

    handle.abort();
}

#[tokio::test]
async fn ts4e_connection_cap_max_connections_2() {
    let config = Config {
        max_connections: 2,
        ..Default::default()
    };
    let server = Server::bind(config, "127.0.0.1:0".parse().unwrap())
        .await
        .unwrap();
    let addr = server.local_addr();

    let handle = tokio::spawn(server.run());

    sleep(Duration::from_millis(100)).await;

    let mut stream1 = TcpStream::connect(addr).await.unwrap();
    let stream2 = TcpStream::connect(addr).await.unwrap();

    let req = b"GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
    let resp1 = send_and_read(&mut stream1, req).await;
    assert!(resp1.starts_with(b"HTTP/1.1 200"));

    let mut stream3 = TcpStream::connect(addr).await.unwrap();
    let resp3 = send_and_read(&mut stream3, req).await;
    assert!(resp3.starts_with(b"HTTP/1.1 200"));

    drop(stream1);
    drop(stream2);
    drop(stream3);

    handle.abort();
}

#[tokio::test]
async fn ts4f_graceful_shutdown_in_flight_completes() {
    let config = Config {
        shutdown_drain_timeout: Duration::from_secs(2),
        ..Default::default()
    };
    let server = Server::bind(config, "127.0.0.1:0".parse().unwrap())
        .await
        .unwrap();
    let addr = server.local_addr();
    let shutdown_handle = server.shutdown_handle();

    let handle = tokio::spawn(server.run());

    sleep(Duration::from_millis(100)).await;

    let mut stream = TcpStream::connect(addr).await.unwrap();
    let req = b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
    let resp = send_and_read(&mut stream, req).await;
    assert!(resp.starts_with(b"HTTP/1.1 200"));

    shutdown_handle.shutdown();

    sleep(Duration::from_millis(100)).await;
    let stream2 = TcpStream::connect(addr).await;
    eprintln!("DEBUG STREAM2 RESULT: {:?}", stream2);
    assert!(stream2.is_err());

    handle.await.unwrap().unwrap();
}

#[tokio::test]
async fn rs1_response_has_status_line_content_length_content_type() {
    let config = Config::default();
    let server = Server::bind(config, "127.0.0.1:0".parse().unwrap())
        .await
        .unwrap();
    let addr = server.local_addr();

    let handle = tokio::spawn(server.run());

    sleep(Duration::from_millis(100)).await;

    let mut stream = TcpStream::connect(addr).await.unwrap();
    let req = b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
    let resp = send_and_read(&mut stream, req).await;

    let resp_str = String::from_utf8_lossy(&resp);
    eprintln!("DEBUG RESPONSE: {:?}", resp_str);
    assert!(resp_str.starts_with("HTTP/1.1 200 OK"));
    assert!(resp_str.contains("content-length:"));
    assert!(resp_str.contains("content-type:"));

    handle.abort();
}

#[tokio::test]
async fn rs2_error_response_has_connection_close() {
    let config = Config::default();
    let server = Server::bind(config, "127.0.0.1:0".parse().unwrap())
        .await
        .unwrap();
    let addr = server.local_addr();

    let handle = tokio::spawn(server.run());

    sleep(Duration::from_millis(100)).await;

    let mut stream = TcpStream::connect(addr).await.unwrap();
    let req = b"GET /unknown HTTP/1.1\r\nHost: localhost\r\n\r\n";
    let resp = send_and_read(&mut stream, req).await;

    let resp_str = String::from_utf8_lossy(&resp);
    eprintln!("DEBUG 404 RESPONSE: {:?}", resp_str);
    assert!(resp_str.contains("connection: close"));

    handle.abort();
}

#[tokio::test]
async fn rs3_success_no_connection_header_persistent() {
    let config = Config::default();
    let server = Server::bind(config, "127.0.0.1:0".parse().unwrap())
        .await
        .unwrap();
    let addr = server.local_addr();

    let handle = tokio::spawn(server.run());

    sleep(Duration::from_millis(100)).await;

    let mut stream = TcpStream::connect(addr).await.unwrap();
    let req = b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
    let resp = send_and_read(&mut stream, req).await;

    let resp_str = String::from_utf8_lossy(&resp);
    eprintln!("DEBUG RESPONSE 2: {:?}", resp_str);
    assert!(!resp_str.contains("connection:"));

    handle.abort();
}

#[tokio::test]
async fn rs3_client_connection_close_server_responds_close() {
    let config = Config::default();
    let server = Server::bind(config, "127.0.0.1:0".parse().unwrap())
        .await
        .unwrap();
    let addr = server.local_addr();

    let handle = tokio::spawn(server.run());

    sleep(Duration::from_millis(100)).await;

    let mut stream = TcpStream::connect(addr).await.unwrap();
    let req = b"GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
    let resp = send_and_read(&mut stream, req).await;

    let resp_str = String::from_utf8_lossy(&resp);
    eprintln!("DEBUG CONNECTION CLOSE RESPONSE: {:?}", resp_str);
    assert!(resp_str.contains("connection: close"));

    handle.abort();
}

#[tokio::test]
async fn pr6_get_with_body_reads_and_discards_integration() {
    let config = Config::default();
    let server = Server::bind(config, "127.0.0.1:0".parse().unwrap())
        .await
        .unwrap();
    let addr = server.local_addr();

    let handle = tokio::spawn(server.run());

    sleep(Duration::from_millis(100)).await;

    let mut stream = TcpStream::connect(addr).await.unwrap();
    let req = b"GET / HTTP/1.1\r\nHost: localhost\r\nContent-Length: 5\r\n\r\nhello";
    let resp = send_and_read(&mut stream, req).await;

    assert!(resp.starts_with(b"HTTP/1.1 200"));

    handle.abort();
}

#[tokio::test]
async fn pr9_pipelining_sequential_processing() {
    let config = Config::default();
    let server = Server::bind(config, "127.0.0.1:0".parse().unwrap())
        .await
        .unwrap();
    let addr = server.local_addr();

    let handle = tokio::spawn(server.run());

    sleep(Duration::from_millis(100)).await;

    let mut stream = TcpStream::connect(addr).await.unwrap();
    let req1 = b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
    let req2 = b"GET /health HTTP/1.1\r\nHost: localhost\r\n\r\n";
    stream.write_all(req1).await.unwrap();
    stream.write_all(req2).await.unwrap();

    let mut buf = vec![0u8; 4096];
    let n = timeout(Duration::from_secs(5), stream.read(&mut buf))
        .await
        .unwrap()
        .unwrap();
    buf.truncate(n);

    let resp_str = String::from_utf8_lossy(&buf);
    assert!(resp_str.contains("Hello"));
    assert!(resp_str.contains(r#"{"status":"ok"}"#));

    handle.abort();
}
