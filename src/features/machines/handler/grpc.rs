//! tonic gRPC handler for machines (Go `handler/handler.go` parity,
//! port-spec §5.2a). Maps `proto/machines/v1/machines.proto` payloads to/from
//! the feature's contract types and calls the usecase service —
//! never domain entities or the repository.
//!
//! With package `machines.v1` and no `rust_package` override,
//! `tonic::include_proto!` emits a module named after the package with dots
//! replaced by underscores: `machines_v1`.

use std::sync::Arc;

use tonic::{Request, Response, Status};

use crate::features::machines::contract::{
    CreateMachineRequest, ListMachinesRequest, MachineResponse, RestockBankRequest,
};
use crate::features::machines::usecase::Service;
use crate::platform::errors::AppError;
use crate::platform::server::GRPC_MESSAGE_SIZE_LIMIT;

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
mod tests;
