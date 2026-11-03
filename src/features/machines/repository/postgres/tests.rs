use super::*;

fn row(bank_json: &str) -> MachineRow {
    MachineRow {
        id: Uuid::nil(),
        label: "lobby".to_string(),
        coin_bank: bank_json.to_string(),
        created_at: OffsetDateTime::UNIX_EPOCH,
        updated_at: OffsetDateTime::UNIX_EPOCH,
    }
}

#[test]
fn jsonb_text_maps_to_domain_bank() {
    let machine = row_to_machine(row(r#"{"25":2,"100":1}"#)).expect("parse");
    assert_eq!(machine.id, Uuid::nil());
    assert_eq!(machine.label, "lobby");
    assert_eq!(machine.coin_bank.get(&25), Some(&2));
    assert_eq!(machine.coin_bank.get(&100), Some(&1));
}

#[test]
fn empty_object_maps_to_empty_bank() {
    let machine = row_to_machine(row("{}")).expect("parse");
    assert!(machine.coin_bank.is_empty());
}

#[test]
fn malformed_jsonb_is_internal() {
    let err = row_to_machine(row("not json")).unwrap_err();
    assert!(matches!(err, Error::Internal { cause: Some(_) }));
}

#[test]
fn row_not_found_translates_to_machine_not_found() {
    assert_eq!(
        map_sqlx_error(sqlx::Error::RowNotFound),
        Error::MachineNotFound
    );
}

#[test]
fn other_sqlx_errors_become_internal() {
    assert!(matches!(
        map_sqlx_error(sqlx::Error::PoolClosed),
        Error::Internal { cause: Some(_) }
    ));
}
