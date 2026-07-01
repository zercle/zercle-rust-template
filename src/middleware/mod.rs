//! HTTP middleware stack. Each submodule exposes a `pub fn layer() -> ...` constructor that
//! returns a tower layer, plus the middleware function itself when applicable.
//!
//! Order applied in `shared/server/http.rs` (structure.md §9, canvas row 26):
//! Recover → RequestID → OTel(TraceLayer) → AccessLog → CORS → BodyLimit.

pub mod access_log;
pub mod cors;
pub mod recover;
pub mod request_id;

pub use access_log::layer as access_log_layer;
pub use cors::{default_layer as default_cors_layer, layer as cors_layer};
pub use recover::layer as recover_layer;
pub use request_id::{RequestId, current as current_request_id, layer as request_id_layer};
