use crate::domain::entities::{CreateUser, User};
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn create(&self, user: &CreateUser) -> anyhow::Result<User>;
    async fn find_by_id(&self, id: Uuid) -> anyhow::Result<Option<User>>;
    async fn find_by_email(&self, email: &str) -> anyhow::Result<Option<User>>;
    async fn update(&self, user: &User) -> anyhow::Result<User>;
    async fn delete(&self, id: Uuid) -> anyhow::Result<()>;
    async fn list(&self, limit: i64, offset: i64) -> anyhow::Result<(Vec<User>, i64)>;
    async fn count(&self) -> anyhow::Result<i64>;
}
