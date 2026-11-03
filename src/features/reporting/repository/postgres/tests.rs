use super::*;

fn sample_overview_row() -> OverviewRow {
    OverviewRow {
        product_count: 2,
        total_stock: 9,
        machine_count: 3,
        total_bank_cents: 265,
        purchase_count: 4,
        revenue_cents: 350,
    }
}

fn sample_machine_row() -> MachineSalesRow {
    MachineSalesRow {
        machine_id: Uuid::from_u128(0xabc),
        label: "lobby".to_string(),
        purchase_count: 2,
        revenue_cents: 300,
    }
}

#[test]
fn overview_row_maps_to_domain() {
    let overview: Overview = sample_overview_row().into();
    assert_eq!(overview.product_count, 2);
    assert_eq!(overview.total_stock, 9);
    assert_eq!(overview.machine_count, 3);
    assert_eq!(overview.total_bank_cents, 265);
    assert_eq!(overview.purchase_count, 4);
    assert_eq!(overview.revenue_cents, 350);
}

#[test]
fn machine_row_maps_to_domain() {
    let machine: MachineSales = sample_machine_row().into();
    assert_eq!(machine.machine_id, Uuid::from_u128(0xabc));
    assert_eq!(machine.label, "lobby");
    assert_eq!(machine.purchase_count, 2);
    assert_eq!(machine.revenue_cents, 300);
}

#[test]
fn sqlx_errors_become_internal() {
    let err = map_sqlx_error(sqlx::Error::PoolClosed);
    assert!(matches!(err, Error::Internal { cause: Some(_) }));
}
