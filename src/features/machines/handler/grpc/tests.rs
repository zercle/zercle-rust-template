use super::*;
use crate::features::machines::contract::ListMachinesResponse;
use crate::features::machines::domain::Error;
use crate::features::machines::usecase::MockService;
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
        .withf(|req: &CreateMachineRequest| req.label == "lobby" && req.initial_coins == vec![25])
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
