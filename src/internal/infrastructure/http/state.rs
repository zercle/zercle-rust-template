use std::sync::Arc;

use crate::internal::domain::user::traits::JwtGenerator;

/// Application state shared across HTTP handlers
#[derive(Clone)]
pub struct AppState {
    /// JWT generator for token validation in middleware
    pub jwt_generator: Arc<dyn JwtGenerator>,
}

impl AppState {
    /// Create new application state
    pub fn new(jwt_generator: Arc<dyn JwtGenerator>) -> Self {
        Self { jwt_generator }
    }
}
