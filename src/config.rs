use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Config {
    pub max_header_bytes: usize,
    pub max_body_bytes: usize,
    pub header_read_timeout: Duration,
    pub body_read_timeout: Duration,
    pub keepalive_idle_timeout: Duration,
    pub max_connections: usize,
    pub shutdown_drain_timeout: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_header_bytes: 8192,
            max_body_bytes: 1_048_576,
            header_read_timeout: Duration::from_secs(10),
            body_read_timeout: Duration::from_secs(10),
            keepalive_idle_timeout: Duration::from_secs(30),
            max_connections: 1024,
            shutdown_drain_timeout: Duration::from_secs(5),
        }
    }
}
