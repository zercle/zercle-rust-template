//! STUB FEATURE — delete src/features/example to start your project.
//!
//! Driving (inbound) adapters: they translate transport payloads to/from the
//! feature's contract types and call the application's inbound port
//! (`application::Service`). They must not touch outbound ports or driven
//! adapters (`tests/architecture.rs`: driving-adapters-ignore-ports-and-driven-adapters).

pub mod grpc;
pub mod http;
