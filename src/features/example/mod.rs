//! STUB FEATURE — delete src/features/example to start your project.
//!
//! Public re-exports + composition helpers for the example feature.

use std::sync::Arc;

use axum::Router;

pub mod domain;
pub mod dto;
pub mod grpc;
pub mod handler;
pub mod repository;
pub mod service;

pub use domain::{Error, Item, Repository, Service, SharedService};
pub use dto::{CreateItemRequest, ItemResponse, ListItemsRequest, ListItemsResponse};
pub use grpc::{GrpcServer, server as grpc_server};
pub use handler::{Handler, routes as handler_routes};
pub use repository::PgRepository;
pub use service::ServiceImpl;

/// Build the axum router for the example feature.
pub fn http_routes<S>(service: Arc<S>) -> Router
where
    S: domain::Service + ?Sized + Send + Sync + 'static,
{
    handler::routes(service)
}
