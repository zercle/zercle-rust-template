//! Driving (inbound) adapters for sales: `http.rs` and `grpc.rs`. They call
//! `application::Service` only.

pub mod grpc;
pub mod http;
