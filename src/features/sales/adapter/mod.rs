//! Interface adapters (outer ring) for sales: `driven` (postgres repository)
//! and `driving` (axum HTTP + tonic gRPC).

pub mod driven;
pub mod driving;
