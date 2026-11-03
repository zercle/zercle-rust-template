use super::*;
use crate::features::machines::domain::CoinBank;
use crate::features::machines::repository::MockRepository;
use mockall::predicate::*;

fn machine(id: Uuid, label: &str, bank: CoinBank) -> Machine {
    let now = OffsetDateTime::now_utc();
    Machine {
        id,
        label: label.to_string(),
        coin_bank: bank,
        created_at: now,
        updated_at: now,
    }
}

fn usecase(repo: MockRepository) -> Usecase {
    Usecase::new(Arc::new(repo), 20, 100, 255)
}

#[tokio::test]
async fn create_rejects_empty_label() {
    let svc = usecase(MockRepository::new());
    assert_eq!(
        svc.create(CreateMachineRequest {
            label: "  ".to_string(),
            initial_coins: vec![],
        })
        .await
        .unwrap_err(),
        Error::InvalidMachineLabel
    );
}

#[tokio::test]
async fn create_rejects_overlong_label() {
    let svc = usecase(MockRepository::new());
    assert_eq!(
        svc.create(CreateMachineRequest {
            label: "a".repeat(256),
            initial_coins: vec![],
        })
        .await
        .unwrap_err(),
        Error::InvalidMachineLabel
    );
}

#[tokio::test]
async fn create_rejects_unsupported_initial_coin() {
    let svc = usecase(MockRepository::new());
    assert_eq!(
        svc.create(CreateMachineRequest {
            label: "lobby".to_string(),
            initial_coins: vec![7],
        })
        .await
        .unwrap_err(),
        Error::UnsupportedCoin
    );
}

#[tokio::test]
async fn create_builds_bank_from_initial_coins() {
    let mut mock = MockRepository::new();
    mock.expect_create().returning(|_| Ok(()));
    let svc = usecase(mock);
    let resp = svc
        .create(CreateMachineRequest {
            label: " lobby ".to_string(),
            initial_coins: vec![25, 25, 100],
        })
        .await
        .unwrap();
    assert_eq!(resp.label, "lobby");
    assert_eq!(resp.coin_bank.get(&25), Some(&2));
    assert_eq!(resp.coin_bank.get(&100), Some(&1));
}

#[tokio::test]
async fn get_rejects_malformed_id_before_touching_the_port() {
    let mut mock = MockRepository::new();
    mock.expect_get_by_id().times(0);
    let svc = usecase(mock);
    assert_eq!(
        svc.get("not-a-uuid".to_string()).await.unwrap_err(),
        Error::InvalidId
    );
}

#[tokio::test]
async fn get_maps_empty_bank_to_empty_object() {
    let mut mock = MockRepository::new();
    mock.expect_get_by_id()
        .with(eq(Uuid::nil()))
        .returning(|id| Ok(machine(id, "lobby", CoinBank::new())));
    let svc = usecase(mock);
    let resp = svc.get(Uuid::nil().to_string()).await.unwrap();
    assert!(resp.coin_bank.is_empty());
}

#[tokio::test]
async fn get_passes_through_not_found() {
    let mut mock = MockRepository::new();
    mock.expect_get_by_id()
        .returning(|_| Err(Error::MachineNotFound));
    let svc = usecase(mock);
    assert_eq!(
        svc.get(Uuid::nil().to_string()).await.unwrap_err(),
        Error::MachineNotFound
    );
}

#[tokio::test]
async fn list_clamps_limit_above_max() {
    let mut mock = MockRepository::new();
    mock.expect_list()
        .withf(|limit, offset| *limit == 100 && *offset == 0)
        .returning(|_, _| Ok(vec![]));
    let svc = usecase(mock);
    let resp = svc
        .list(ListMachinesRequest {
            limit: Some(9_999),
            offset: Some(0),
        })
        .await
        .unwrap();
    assert!(resp.machines.is_empty());
}

#[tokio::test]
async fn list_uses_default_and_clamps_negative_offset() {
    let mut mock = MockRepository::new();
    mock.expect_list()
        .withf(|limit, offset| *limit == 20 && *offset == 0)
        .returning(|_, _| Ok(vec![]))
        .times(2);
    let svc = usecase(mock);
    svc.list(ListMachinesRequest {
        limit: None,
        offset: None,
    })
    .await
    .unwrap();
    svc.list(ListMachinesRequest {
        limit: Some(0),
        offset: Some(-9),
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn fallback_defaults_apply_when_config_zero() {
    let mut mock = MockRepository::new();
    mock.expect_list()
        .withf(|limit, offset| *limit == 100 && *offset == 0)
        .returning(|_, _| Ok(vec![]));
    let svc = Usecase::new(Arc::new(mock), 0, 0, 0);
    svc.list(ListMachinesRequest {
        limit: Some(9_999),
        offset: Some(0),
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn restock_rejects_unsupported_coin_before_port() {
    let mut mock = MockRepository::new();
    mock.expect_restock_bank().times(0);
    let svc = usecase(mock);
    assert_eq!(
        svc.restock_bank(
            Uuid::nil().to_string(),
            RestockBankRequest { coins: vec![7] },
        )
        .await
        .unwrap_err(),
        Error::UnsupportedCoin
    );
}

#[tokio::test]
async fn restock_rejects_malformed_id_before_port() {
    let mut mock = MockRepository::new();
    mock.expect_restock_bank().times(0);
    let svc = usecase(mock);
    assert_eq!(
        svc.restock_bank(
            "not-a-uuid".to_string(),
            RestockBankRequest { coins: vec![25] },
        )
        .await
        .unwrap_err(),
        Error::InvalidId
    );
}

#[tokio::test]
async fn restock_re_reads_the_authoritative_bank() {
    let mut mock = MockRepository::new();
    mock.expect_restock_bank()
        .with(eq(Uuid::nil()), eq(vec![25]))
        .returning(|_, _| Ok(()));
    let mut bank = CoinBank::new();
    bank.insert(25, 3);
    mock.expect_get_by_id()
        .with(eq(Uuid::nil()))
        .returning(move |id| Ok(machine(id, "lobby", bank.clone())));
    let svc = usecase(mock);
    let resp = svc
        .restock_bank(
            Uuid::nil().to_string(),
            RestockBankRequest { coins: vec![25] },
        )
        .await
        .unwrap();
    assert_eq!(resp.coin_bank.get(&25), Some(&3));
}
