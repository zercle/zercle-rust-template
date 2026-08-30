//! STUB FEATURE — delete src/features/example to start your project.
//!
//! Interface adapters (clean-architecture outer ring): driving adapters under
//! `driving` (HTTP, gRPC — Go `adapter/in`) and driven adapters under
//! `driven` (persistence — Go `adapter/out`). (`in` is a reserved keyword in
//! Rust, hence `driving`/`driven` — standard hexagonal terminology for the
//! same halves.)

pub mod driven;
pub mod driving;
