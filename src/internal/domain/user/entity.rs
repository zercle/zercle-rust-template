use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub full_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl User {
    pub fn new(id: Uuid, email: String, password_hash: String, full_name: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            id,
            email,
            password_hash,
            full_name,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl RefreshToken {
    pub fn new(id: Uuid, user_id: Uuid, token: String, expires_at: DateTime<Utc>) -> Self {
        let now = Utc::now();
        Self {
            id,
            user_id,
            token,
            expires_at,
            created_at: now,
        }
    }
}
