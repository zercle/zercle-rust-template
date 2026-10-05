//! tonic gRPC driving adapter for machines (Go `handler/handler.go` parity,
//! port-spec §5.2a). Maps `proto/machines/v1/machines.proto` payloads to/from
//! the feature's contract types and calls the application's inbound port —
//! never domain entities or the repository.
//!
//! With package `machines.v1` and no `rust_package` override,
//! `tonic::include_proto!` emits a module named after the package with dots
//! replaced by underscores: `machines_v1`.

use std::sync::Arc;

use tonic::{Request, Response, Status};

use crate::features::machines::application::Service;
use crate::features::machines::contract::{
    CreateMachineRequest, ListMachinesRequest, MachineResponse, RestockBankRequest,
};
use crate::platform::errors::AppError;

/// 4 MiB cap on incoming + outgoing gRPC message bodies (Go
/// `grpc.MaxRecvMsgSize(4*1024*1024)` / `MaxSendMsgSize(4*1024*1024)` parity;
/// owned by the feature because it protects this service's payloads).
const GRPC_MESSAGE_SIZE_LIMIT: usize = 4 * 1024 * 1024;

// The generated server trait re-applies `#[must_use]` on futures that already
// carry it; the outer allow keeps `-D warnings` clean without touching codegen.
#[allow(clippy::double_must_use)]
pub mod machines_v1 {
    tonic::include_proto!("machines.v1");
}

use machines_v1::{
    CreateMachineRequest as PbCreateMachineRequest, GetMachineRequest,
    ListMachinesRequest as PbListMachinesRequest, ListMachinesResponse as PbListMachinesResponse,
    Machine as PbMachine, RestockBankRequest as PbRestockBankRequest,
    machines_service_server::{MachinesService, MachinesServiceServer},
};

/// tonic server implementation of the machines feature. Generic over the
/// inbound port so tests inject a `MockService` and the `di` wires
/// `Arc<dyn Service>`.
pub struct GrpcServer<S: Service + ?Sized> {
    service: Arc<S>,
}

// Manual Clone impl: `Arc<S>` is `Clone` for any `S`, including `?Sized`.
impl<S: Service + ?Sized> Clone for GrpcServer<S> {
    fn clone(&self) -> Self {
        Self {
            service: self.service.clone(),
        }
    }
}

impl<S: Service + ?Sized + 'static> GrpcServer<S> {
    pub fn new(service: Arc<S>) -> Self {
        Self { service }
    }
}

#[tonic::async_trait]
impl<S: Service + ?Sized + 'static> MachinesService for GrpcServer<S> {
    async fn create_machine(
        &self,
        request: Request<PbCreateMachineRequest>,
    ) -> Result<Response<PbMachine>, Status> {
        let req = request.into_inner();
        let resp = self
            .service
            .create(CreateMachineRequest {
                label: req.label,
                initial_coins: req.initial_coins,
            })
            .await
            .map_err(|e| AppError::from(e).to_grpc_status())?;
        Ok(Response::new(contract_to_pb(&resp)))
    }

    async fn get_machine(
        &self,
        request: Request<GetMachineRequest>,
    ) -> Result<Response<PbMachine>, Status> {
        let id = request.into_inner().id;
        let resp = self
            .service
            .get(id)
            .await
            .map_err(|e| AppError::from(e).to_grpc_status())?;
        Ok(Response::new(contract_to_pb(&resp)))
    }

    async fn list_machines(
        &self,
        request: Request<PbListMachinesRequest>,
    ) -> Result<Response<PbListMachinesResponse>, Status> {
        let req = request.into_inner();
        let resp = self
            .service
            .list(ListMachinesRequest {
                limit: Some(req.limit),
                offset: Some(req.offset),
            })
            .await
            .map_err(|e| AppError::from(e).to_grpc_status())?;
        Ok(Response::new(PbListMachinesResponse {
            machines: resp.machines.iter().map(contract_to_pb).collect(),
        }))
    }

    async fn restock_bank(
        &self,
        request: Request<PbRestockBankRequest>,
    ) -> Result<Response<PbMachine>, Status> {
        let req = request.into_inner();
        let resp = self
            .service
            .restock_bank(req.id, RestockBankRequest { coins: req.coins })
            .await
            .map_err(|e| AppError::from(e).to_grpc_status())?;
        Ok(Response::new(contract_to_pb(&resp)))
    }
}

/// Build a tonic service suitable for `Server::add_service`, with the message
/// size limits applied.
pub fn server<S: Service + ?Sized + 'static>(
    srv: GrpcServer<S>,
) -> MachinesServiceServer<GrpcServer<S>> {
    MachinesServiceServer::new(srv)
        .max_decoding_message_size(GRPC_MESSAGE_SIZE_LIMIT)
        .max_encoding_message_size(GRPC_MESSAGE_SIZE_LIMIT)
}

fn contract_to_pb(resp: &MachineResponse) -> PbMachine {
    PbMachine {
        id: resp.id.clone(),
        label: resp.label.clone(),
        coin_bank: resp.coin_bank.iter().map(|(k, v)| (*k, *v)).collect(),
        created_at: resp.created_at.clone(),
        updated_at: resp.updated_at.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::machines::application::MockService;
    use crate::features::machines::contract::ListMachinesResponse;
    use crate::features::machines::domain::Error;
    use mockall::predicate::*;

    fn sample(id: &str, label: &str) -> MachineResponse {
        let mut coin_bank = std::collections::BTreeMap::new();
        coin_bank.insert(25, 2);
        MachineResponse {
            id: id.to_string(),
            label: label.to_string(),
            coin_bank,
            created_at: "1970-01-01T00:00:00Z".to_string(),
            updated_at: "1970-01-01T00:00:00Z".to_string(),
        }
    }

    #[tokio::test]
    async fn create_machine_ok() {
        let mut m = MockService::new();
        m.expect_create()
            .withf(|req: &CreateMachineRequest| {
                req.label == "lobby" && req.initial_coins == vec![25]
            })
            .returning(|req| Ok(sample("some-id", &req.label)));
        let srv = GrpcServer::new(Arc::new(m));
        let resp = srv
            .create_machine(Request::new(PbCreateMachineRequest {
                label: "lobby".to_string(),
                initial_coins: vec![25],
            }))
            .await
            .unwrap();
        let machine = resp.into_inner();
        assert_eq!(machine.label, "lobby");
        assert_eq!(machine.coin_bank.get(&25), Some(&2));
    }

    #[tokio::test]
    async fn create_machine_maps_invalid_label_to_invalid_argument() {
        let mut m = MockService::new();
        m.expect_create()
            .returning(|_| Err(Error::InvalidMachineLabel));
        let srv = GrpcServer::new(Arc::new(m));
        let err = srv
            .create_machine(Request::new(PbCreateMachineRequest {
                label: String::new(),
                initial_coins: vec![],
            }))
            .await
            .unwrap_err();
        assert_eq!(err.code(), tonic::Code::InvalidArgument);
    }

    #[tokio::test]
    async fn get_machine_not_found_maps_to_not_found() {
        let mut m = MockService::new();
        m.expect_get().returning(|_| Err(Error::MachineNotFound));
        let srv = GrpcServer::new(Arc::new(m));
        let err = srv
            .get_machine(Request::new(GetMachineRequest {
                id: uuid::Uuid::nil().to_string(),
            }))
            .await
            .unwrap_err();
        assert_eq!(err.code(), tonic::Code::NotFound);
    }

    #[tokio::test]
    async fn get_machine_bad_uuid_maps_to_invalid_argument() {
        let mut m = MockService::new();
        m.expect_get().returning(|_| Err(Error::InvalidId));
        let srv = GrpcServer::new(Arc::new(m));
        let err = srv
            .get_machine(Request::new(GetMachineRequest {
                id: "not-a-uuid".to_string(),
            }))
            .await
            .unwrap_err();
        assert_eq!(err.code(), tonic::Code::InvalidArgument);
    }

    #[tokio::test]
    async fn list_machines_ok() {
        let mut m = MockService::new();
        m.expect_list()
            .withf(|req: &ListMachinesRequest| req.limit == Some(10) && req.offset == Some(0))
            .returning(|_| {
                Ok(ListMachinesResponse {
                    machines: vec![sample("a", "one"), sample("b", "two")],
                })
            });
        let srv = GrpcServer::new(Arc::new(m));
        let resp = srv
            .list_machines(Request::new(PbListMachinesRequest {
                limit: 10,
                offset: 0,
            }))
            .await
            .unwrap();
        assert_eq!(resp.into_inner().machines.len(), 2);
    }

    #[tokio::test]
    async fn restock_bank_ok() {
        let mut m = MockService::new();
        m.expect_restock_bank()
            .withf(|id, req| id == "00000000-0000-0000-0000-000000000000" && req.coins == vec![100])
            .returning(|id, _| Ok(sample(&id, "lobby")));
        let srv = GrpcServer::new(Arc::new(m));
        let resp = srv
            .restock_bank(Request::new(PbRestockBankRequest {
                id: uuid::Uuid::nil().to_string(),
                coins: vec![100],
            }))
            .await
            .unwrap();
        assert_eq!(resp.into_inner().id, uuid::Uuid::nil().to_string());
    }

    #[tokio::test]
    async fn restock_bank_unsupported_coin_maps_to_invalid_argument() {
        let mut m = MockService::new();
        m.expect_restock_bank()
            .returning(|_, _| Err(Error::UnsupportedCoin));
        let srv = GrpcServer::new(Arc::new(m));
        let err = srv
            .restock_bank(Request::new(PbRestockBankRequest {
                id: uuid::Uuid::nil().to_string(),
                coins: vec![7],
            }))
            .await
            .unwrap_err();
        assert_eq!(err.code(), tonic::Code::InvalidArgument);
    }
}
