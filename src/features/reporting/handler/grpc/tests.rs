use super::*;
use crate::features::reporting::contract::{CatalogStats, MachineStats, SalesStats};
use crate::features::reporting::usecase::MockService;

fn sample_response() -> SummaryResponse {
    SummaryResponse {
        catalog: CatalogStats {
            product_count: 2,
            total_stock: 9,
        },
        machines: MachineStats {
            machine_count: 1,
            total_coin_bank_cents: 250,
        },
        sales: SalesStats {
            purchase_count: 3,
            revenue_cents: 300,
        },
        top_machines: vec![crate::features::reporting::contract::MachineSales {
            machine_id: "11111111-1111-1111-1111-111111111111".to_string(),
            label: "lobby".to_string(),
            purchase_count: 2,
            revenue_cents: 300,
        }],
    }
}

#[tokio::test]
async fn summary_ok_maps_contract_to_proto() {
    let mut m = MockService::new();
    m.expect_summary()
        .withf(|req: &SummaryRequest| req.top == Some(3))
        .returning(|_| Ok(sample_response()));
    let srv = GrpcServer::new(Arc::new(m));

    let resp = srv
        .summary(Request::new(PbSummaryRequest { top: 3 }))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(resp.catalog.unwrap().product_count, 2);
    assert_eq!(resp.machines.unwrap().total_coin_bank_cents, 250);
    assert_eq!(resp.sales.unwrap().revenue_cents, 300);
    assert_eq!(resp.top_machines.len(), 1);
    assert_eq!(resp.top_machines[0].label, "lobby");
}

#[tokio::test]
async fn summary_maps_invalid_top_to_invalid_argument() {
    let mut m = MockService::new();
    m.expect_summary()
        .returning(|_| Err(crate::features::reporting::domain::Error::InvalidTopMachines));
    let srv = GrpcServer::new(Arc::new(m));

    let err = srv
        .summary(Request::new(PbSummaryRequest { top: 999 }))
        .await
        .unwrap_err();
    assert_eq!(err.code(), tonic::Code::InvalidArgument);
}
