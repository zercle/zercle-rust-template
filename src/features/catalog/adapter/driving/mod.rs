//! Driving (inbound) adapters for catalog: `http.rs` (axum, Go
//! `handler/handler.go` parity) and `grpc.rs` (tonic). Both call
//! `application::Service` only — never the outbound port or driven adapters.

pub mod grpc;
pub mod http;
