use crate::config::Config;
use crate::parser::{parse, Limits, ParseOutcome};
use crate::response::serialize;
use crate::router::{route, Response};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Semaphore};
use tokio::time::timeout;

pub struct Server {
    config: Config,
    listener: Arc<TcpListener>,
    semaphore: Arc<Semaphore>,
    shutdown_tx: mpsc::Sender<()>,
    shutdown_rx: mpsc::Receiver<()>,
}

impl Server {
    pub async fn bind(config: Config, addr: SocketAddr) -> std::io::Result<Self> {
        let listener = TcpListener::bind(addr).await?;
        let _local_addr = listener.local_addr()?;
        let listener = Arc::new(listener);
        let semaphore = Arc::new(Semaphore::new(config.max_connections));
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        Ok(Self {
            config,
            listener,
            semaphore,
            shutdown_tx,
            shutdown_rx,
        })
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.listener.local_addr().unwrap()
    }

    pub async fn run(self) -> std::io::Result<()> {
        let mut shutdown_rx = self.shutdown_rx;
        let config = Arc::new(self.config);
        let semaphore = self.semaphore.clone();
        let listener = self.listener;

        let (drain_tx, mut drain_rx) = mpsc::channel::<()>(1024);

        let accept_loop = async {
            loop {
                let permit = match semaphore.clone().acquire_owned().await {
                    Ok(p) => p,
                    Err(_) => break,
                };

                let (stream, peer_addr) = match listener.accept().await {
                    Ok(s) => s,
                    Err(_) => {
                        drop(permit);
                        break;
                    }
                };

                let config = config.clone();
                let drain_tx = drain_tx.clone();

                tokio::spawn(async move {
                    let _permit = permit;
                    if let Err(e) = handle_connection(stream, peer_addr, config, drain_tx).await {
                        eprintln!("Connection error: {}", e);
                    }
                });
            }
        };

        tokio::select! {
            _ = shutdown_rx.recv() => {
                eprintln!("DEBUG: Shutdown signal received, dropping listener");
                drop(listener);
            }
            _ = accept_loop => {
                eprintln!("DEBUG: Accept loop finished unexpectedly");
            }
        }

        eprintln!("DEBUG: Shutdown complete, waiting for drain");
        drop(drain_tx);
        let drain_timeout = config.shutdown_drain_timeout;
        let _ = timeout(drain_timeout, async {
            while drain_rx.recv().await.is_some() {}
        })
        .await;

        eprintln!("DEBUG: Server fully stopped");
        Ok(())
    }

    pub fn shutdown_handle(&self) -> ShutdownHandle {
        ShutdownHandle {
            tx: self.shutdown_tx.clone(),
        }
    }
}

pub struct ShutdownHandle {
    tx: mpsc::Sender<()>,
}

impl ShutdownHandle {
    pub fn shutdown(&self) {
        let _ = self.tx.try_send(());
    }
}

async fn handle_connection(
    mut stream: TcpStream,
    peer_addr: SocketAddr,
    config: Arc<Config>,
    drain_tx: mpsc::Sender<()>,
) -> std::io::Result<()> {
    let mut buf = Vec::with_capacity(8192);
    let mut temp_buf = [0u8; 4096];
    let limits = Limits::from(config.as_ref());
    let mut keep_alive = true;
    let mut reading_headers = true;

    loop {
        let read_timeout = if reading_headers {
            config.header_read_timeout
        } else {
            config.body_read_timeout
        };

        let n = match timeout(read_timeout, stream.read(&mut temp_buf)).await {
            Ok(Ok(0)) => break,
            Ok(Ok(n)) => n,
            Ok(Err(e)) => return Err(e),
            Err(_) => {
                let resp = Response::request_timeout();
                let _ = stream.write_all(&serialize(&resp)).await;
                return Ok(());
            }
        };

        buf.extend_from_slice(&temp_buf[..n]);

        let mut cursor = 0;
        let mut request_complete = false;

        while cursor < buf.len() {
            let parse_buf = &buf[cursor..];
            let start = Instant::now();
            let outcome = parse(parse_buf, &limits);
            let duration = start.elapsed();

            match outcome {
                ParseOutcome::Complete { request, consumed } => {
                    reading_headers = false;
                    request_complete = true;
                    let client_close = request
                        .headers
                        .iter()
                        .any(|(k, v)| k == "connection" && v.eq_ignore_ascii_case("close"));
                    let resp = match route(&request) {
                        Ok(r) => {
                            if r.status < 400 {
                                r.with_connection_close(client_close || !keep_alive)
                            } else {
                                r
                            }
                        }
                        Err(_) => Response::internal_server_error(),
                    };
                    let should_close = client_close
                        || resp
                            .headers
                            .iter()
                            .any(|(k, v)| k == "connection" && v == "close");

                    eprintln!(
                        "{} {} {} {} {}",
                        peer_addr,
                        request.method.as_str(),
                        request.path,
                        resp.status,
                        duration.as_millis()
                    );

                    let resp_bytes = serialize(&resp);
                    stream.write_all(&resp_bytes).await?;

                    cursor += consumed;
                    keep_alive = !should_close;

                    if should_close {
                        break;
                    }
                }
                ParseOutcome::Incomplete => {
                    reading_headers = true;
                    break;
                }
                ParseOutcome::Error(e) => {
                    let status = e.status_code();
                    let resp = match status {
                        400 => Response::bad_request(),
                        408 => Response::request_timeout(),
                        411 => Response::length_required(),
                        413 => Response::payload_too_large(),
                        417 => Response::expectation_failed(),
                        431 => Response::header_fields_too_large(),
                        500 => Response::internal_server_error(),
                        501 => Response::not_implemented(),
                        505 => Response::version_not_supported(),
                        _ => Response::internal_server_error(),
                    };
                    eprintln!("{} - - {} {}", peer_addr, status, duration.as_millis());
                    let _ = stream.write_all(&serialize(&resp)).await;
                    return Ok(());
                }
            }
        }

        if cursor > 0 {
            buf.drain(..cursor);
        }

        if request_complete {
            reading_headers = false;
        }

        if !keep_alive {
            break;
        }

        if buf.len() > limits.max_header_bytes {
            let resp = Response::header_fields_too_large();
            let _ = stream.write_all(&serialize(&resp)).await;
            break;
        }
    }

    let _ = drain_tx.send(()).await;
    Ok(())
}
