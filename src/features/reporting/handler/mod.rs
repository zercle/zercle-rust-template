//! Inbound handlers for reporting: `http.rs` and `grpc.rs`. They call
//! `usecase::Service` only.

pub mod grpc;
pub mod http;
