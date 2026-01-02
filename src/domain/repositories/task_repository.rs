use crate::domain::entities::{CreateTask, Task};
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait TaskRepository: Send + Sync {
    async fn create(&self, task: &CreateTask) -> anyhow::Result<Task>;
    async fn find_by_id(&self, id: Uuid) -> anyhow::Result<Option<Task>>;
    async fn find_by_user_id(
        &self,
        user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> anyhow::Result<(Vec<Task>, i64)>;
    async fn update(&self, task: &Task) -> anyhow::Result<Task>;
    async fn delete(&self, id: Uuid, user_id: Uuid) -> anyhow::Result<()>;
    async fn count_by_user_id(&self, user_id: Uuid) -> anyhow::Result<i64>;
}
