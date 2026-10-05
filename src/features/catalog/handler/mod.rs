//! Inbound handlers for catalog: `http.rs` (axum, Go `handler/handler.go`
//! parity) and `grpc.rs` (tonic). Both call `usecase::Service` only — never the
//! repository interface or its postgres implementation.

pub mod grpc;
pub mod http;
