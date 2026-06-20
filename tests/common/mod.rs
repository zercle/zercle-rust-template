//! Shared helpers for integration + e2e tests.
//!
//! These tests require a real Postgres + Valkey pair (the same docker-compose
//! services the binary expects). `infra_reachable` mirrors Go's
//! `infraReachable`/`tcpReachable` and lets each test self-skip cleanly when
//! the operator has not started the containers — `cargo test` is then green
//! in CI without `-infra` and still exercises the live DB when it is.

use std::net::ToSocketAddrs;
use std::time::Duration;

use zercle_rust_template::Config;

/// True iff both `cfg.db.host:port` and `cfg.valkey.host:port` accept a TCP
/// connection within the per-probe timeout.
pub fn infra_reachable(cfg: &Config) -> bool {
    let db = format!("{}:{}", cfg.db.host, cfg.db.port);
    let valkey = format!("{}:{}", cfg.valkey.host, cfg.valkey.port);
    tcp_reachable(&db, Duration::from_secs(2)) && tcp_reachable(&valkey, Duration::from_secs(2))
}

fn tcp_reachable(addr: &str, timeout: Duration) -> bool {
    let sock = match addr.to_socket_addrs() {
        Ok(mut it) => match it.next() {
            Some(s) => s,
            None => return false,
        },
        Err(_) => return false,
    };
    std::net::TcpStream::connect_timeout(&sock, timeout).is_ok()
}

/// Return a JSON-pretty-print helper — kept tiny so the tests can `assert_eq!`
/// on serialized bodies without depending on a heavy helper crate.
#[allow(dead_code)]
pub fn pretty_json<T: serde::Serialize>(v: &T) -> String {
    serde_json::to_string_pretty(v).unwrap_or_else(|_| String::from("<unserializable>"))
}
