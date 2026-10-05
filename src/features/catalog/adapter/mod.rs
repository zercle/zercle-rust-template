//! Interface adapters (outer ring) for catalog. Driving adapters live under
//! `driving` (axum HTTP, tonic gRPC); driven adapters under `driven`
//! (postgres + cache-aside). (`in` is a Rust keyword, hence
//! `driving`/`driven`.)

pub mod driven;
pub mod driving;

#[cfg(test)]
mod integration;
