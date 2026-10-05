//! Interface adapters (outer ring) for catalog. Driving adapters live under
//! `driving` (axum HTTP, tonic gRPC); driven adapters under `driven`
//! (postgres). Both are empty skeletons this wave — the next wave fills them.
//! (`in` is a Rust keyword, hence `driving`/`driven`.)

pub mod driven;
pub mod driving;
