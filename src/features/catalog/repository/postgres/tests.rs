use super::*;

fn sample_row() -> ProductRow {
    ProductRow {
        id: Uuid::nil(),
        name: "alpha".to_string(),
        price_cents: 150,
        stock: 3,
        created_at: OffsetDateTime::UNIX_EPOCH,
        updated_at: OffsetDateTime::UNIX_EPOCH,
    }
}

#[test]
fn row_maps_to_domain_product() {
    let product: Product = sample_row().into();
    assert_eq!(product.id, Uuid::nil());
    assert_eq!(product.name, "alpha");
    assert_eq!(product.price_cents, 150);
    assert_eq!(product.stock, 3);
    assert_eq!(product.created_at, OffsetDateTime::UNIX_EPOCH);
}

#[test]
fn infrastructure_errors_become_internal() {
    let err = map_sqlx_error(sqlx::Error::PoolClosed);
    assert!(matches!(err, Error::Internal { cause: Some(_) }));
}
