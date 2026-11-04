use super::*;

fn sample(id: Uuid) -> Product {
    let mut p = Product {
        id,
        name: "cola".to_string(),
        price_cents: 150,
        stock: 7,
        created_at: OffsetDateTime::UNIX_EPOCH,
        updated_at: OffsetDateTime::UNIX_EPOCH,
    };
    p.created_at = OffsetDateTime::from_unix_timestamp(1_700_000_000).expect("ts");
    p.updated_at = p.created_at;
    p
}

#[test]
fn cached_payload_round_trips_through_json() {
    let product = sample(Uuid::nil());
    let json = serde_json::to_string(&CachedProduct::from_domain(&product)).unwrap();
    assert!(
        json.contains("\"created_at\":\"2023-11-14T22:13:20Z\""),
        "{json}"
    );
    let back = serde_json::from_str::<CachedProduct>(&json).unwrap();
    assert_eq!(back.to_domain().unwrap(), product);
}

#[test]
fn corrupt_id_is_rejected_on_decode() {
    let cached = CachedProduct {
        id: "not-a-uuid".to_string(),
        name: "x".to_string(),
        price_cents: 1,
        stock: 1,
        created_at: OffsetDateTime::UNIX_EPOCH,
        updated_at: OffsetDateTime::UNIX_EPOCH,
    };
    assert_eq!(cached.to_domain().unwrap_err(), Error::InvalidId);
}
