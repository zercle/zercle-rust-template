use super::*;
use crate::features::reporting::repository::MockRepository;
use mockall::predicate::*;
use uuid::Uuid;

fn overview() -> Overview {
    Overview {
        product_count: 2,
        total_stock: 9,
        machine_count: 1,
        total_bank_cents: 250,
        purchase_count: 3,
        revenue_cents: 300,
    }
}

fn usecase(repo: MockRepository) -> Usecase {
    Usecase::new(Arc::new(repo), 5, 20)
}

#[tokio::test]
async fn summary_uses_default_when_top_absent_or_zero() {
    let mut mock = MockRepository::new();
    mock.expect_get_overview().returning(|| Ok(overview()));
    mock.expect_get_top_machines()
        .with(eq(5))
        .returning(|_| Ok(vec![]))
        .times(2);
    let svc = usecase(mock);
    svc.summary(SummaryRequest { top: None }).await.unwrap();
    svc.summary(SummaryRequest { top: Some(0) }).await.unwrap();
}

#[tokio::test]
async fn summary_rejects_top_above_max_before_any_repository_call() {
    let mut mock = MockRepository::new();
    mock.expect_get_overview().times(0);
    mock.expect_get_top_machines().times(0);
    let svc = usecase(mock);
    assert_eq!(
        svc.summary(SummaryRequest { top: Some(21) })
            .await
            .unwrap_err(),
        Error::InvalidTopMachines
    );
}

#[tokio::test]
async fn summary_maps_overview_and_top_machines() {
    let mut mock = MockRepository::new();
    mock.expect_get_overview().returning(|| Ok(overview()));
    mock.expect_get_top_machines().with(eq(3)).returning(|_| {
        Ok(vec![MachineSales {
            machine_id: Uuid::nil(),
            label: "lobby".to_string(),
            purchase_count: 4,
            revenue_cents: 500,
        }])
    });
    let svc = usecase(mock);
    let resp = svc.summary(SummaryRequest { top: Some(3) }).await.unwrap();
    assert_eq!(resp.catalog.product_count, 2);
    assert_eq!(resp.catalog.total_stock, 9);
    assert_eq!(resp.machines.machine_count, 1);
    assert_eq!(resp.machines.total_coin_bank_cents, 250);
    assert_eq!(resp.sales.purchase_count, 3);
    assert_eq!(resp.sales.revenue_cents, 300);
    assert_eq!(resp.top_machines.len(), 1);
    assert_eq!(resp.top_machines[0].machine_id, Uuid::nil().to_string());
    assert_eq!(resp.top_machines[0].label, "lobby");
}

#[tokio::test]
async fn summary_accepts_exactly_max() {
    let mut mock = MockRepository::new();
    mock.expect_get_overview().returning(|| Ok(overview()));
    mock.expect_get_top_machines()
        .with(eq(20))
        .returning(|_| Ok(vec![]));
    let svc = usecase(mock);
    svc.summary(SummaryRequest { top: Some(20) }).await.unwrap();
}

#[tokio::test]
async fn fallback_defaults_apply_when_config_zero() {
    let mut mock = MockRepository::new();
    mock.expect_get_overview().returning(|| Ok(overview()));
    mock.expect_get_top_machines()
        .with(eq(5))
        .returning(|_| Ok(vec![]));
    let svc = Usecase::new(Arc::new(mock), 0, 0);
    svc.summary(SummaryRequest { top: None }).await.unwrap();
}

#[tokio::test]
async fn summary_propagates_repository_errors() {
    let mut mock = MockRepository::new();
    mock.expect_get_overview().returning(|| {
        Err(Error::Internal {
            cause: Some(anyhow::anyhow!("boom")),
        })
    });
    let svc = usecase(mock);
    assert!(matches!(
        svc.summary(SummaryRequest { top: None }).await,
        Err(Error::Internal { .. })
    ));
}
