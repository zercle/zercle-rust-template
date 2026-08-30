//! STUB FEATURE — delete src/features/example to start your project.
//!
//! Composition point for the example feature (Go `di.Register` parity): builds
//! the driven adapter, the use case, and the driving adapters, and registers
//! the domain sentinel → boundary error mapping.

use std::sync::Arc;

use axum::Router;
use sqlx::PgPool;

use crate::features::example::adapter::driven::postgres::PgRepository;
use crate::features::example::adapter::driving::{grpc, http};
use crate::features::example::application::{Service, Usecase};
use crate::features::example::domain::Error;
use crate::platform::config::Config;
use crate::platform::errors::AppError;
use crate::platform::server::GrpcRouter;

/// Sentinel → boundary error mapping, registered here at the composition edge
/// so the domain layer stays dependency-free (Go
/// `apperrors.RegisterSentinel(domain.ErrX, apperrors.ErrY)` parity; the impl
/// is crate-global once defined, so every adapter can rely on `AppError::from`).
impl From<Error> for AppError {
    fn from(err: Error) -> Self {
        match err {
            Error::NotFound => AppError::NotFound { cause: None },
            Error::InvalidName | Error::InvalidId => AppError::InvalidInput { cause: None },
            Error::Internal { cause } => AppError::Internal { cause },
        }
    }
}

/// Everything the feature contributes to the running application.
pub struct Wired {
    /// axum routes for this feature, pre-nested under `/api/v1`.
    pub http: Router,
    /// tonic router for this feature (platform shell serves it as-is).
    pub grpc: GrpcRouter,
}

/// Wire the example feature: postgres repository → use case → HTTP + gRPC
/// adapters. Mirrors Go `di.Register`.
pub fn register(cfg: &Config, db: PgPool) -> Wired {
    let repo = Arc::new(PgRepository::new(db));
    let service: Arc<dyn Service> = Arc::new(Usecase::new(
        repo,
        clamp_i32(cfg.example.default_page_size),
        clamp_i32(cfg.example.max_page_size),
        clamp_i32(cfg.example.max_name_length),
    ));

    let http = Router::new().nest("/api/v1", http::routes(service.clone()));
    let grpc = crate::platform::server::grpc_server()
        .add_service(grpc::server(grpc::GrpcServer::new(service)));

    Wired { http, grpc }
}

/// Config page sizes are validated `>= 1` u32s; clamp to i32 for the proto /
/// port boundary (Go carries them as int32).
fn clamp_i32(v: u32) -> i32 {
    i32::try_from(v).unwrap_or(i32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::errors::errcodes;

    #[test]
    fn domain_sentinels_map_to_boundary_codes() {
        // Assert the registered sentinel mapping (the From impl above) lands
        // on the published error codes.
        let code = |e: Error| AppError::from(e).code().to_string();
        assert_eq!(code(Error::NotFound), errcodes::NOT_FOUND);
        assert_eq!(code(Error::InvalidName), errcodes::INVALID_INPUT);
        assert_eq!(code(Error::InvalidId), errcodes::INVALID_INPUT);
        assert_eq!(code(Error::Internal { cause: None }), errcodes::INTERNAL);
    }
}
