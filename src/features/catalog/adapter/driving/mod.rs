//! Driving (inbound) adapters for catalog: `http.rs` (axum, Go
//! `handler/handler.go` parity) and `grpc.rs` (tonic). Both land in the next
//! wave; they will call `application::Service` only.

pub mod grpc;
pub mod http;
