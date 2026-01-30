use uuid::Uuid;
use zercle_rust_template::internal::domain::user::entity::User;

#[test]
fn test_user_creation() {
    let id = Uuid::new_v4();
    let user = User::new(
        id.clone(),
        "test@example.com".to_string(),
        "hashed_password".to_string(),
        Some("Test User".to_string()),
    );

    assert_eq!(user.id, id);
    assert_eq!(user.email, "test@example.com");
    assert_eq!(user.password_hash, "hashed_password");
    assert_eq!(user.full_name, Some("Test User".to_string()));
    assert!(user.created_at <= user.updated_at);
}

#[test]
fn test_user_creation_without_full_name() {
    let id = Uuid::new_v4();
    let user = User::new(
        id.clone(),
        "test@example.com".to_string(),
        "hashed_password".to_string(),
        None,
    );

    assert_eq!(user.id, id);
    assert_eq!(user.email, "test@example.com");
    assert_eq!(user.full_name, None);
}

#[test]
fn test_user_equality() {
    let id = Uuid::new_v4();
    let user1 = User::new(
        id.clone(),
        "test@example.com".to_string(),
        "hashed_password".to_string(),
        Some("Test User".to_string()),
    );

    let user2 = User::new(
        id,
        "test@example.com".to_string(),
        "hashed_password".to_string(),
        Some("Test User".to_string()),
    );

    assert_eq!(user1.id, user2.id);
    assert_eq!(user1.email, user2.email);
    assert_eq!(user1.password_hash, user2.password_hash);
}
