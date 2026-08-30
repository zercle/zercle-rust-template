//! STUB FEATURE — delete src/features/example to start your project.

use time::OffsetDateTime;
use uuid::Uuid;

/// The trivial example entity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub id: Uuid,
    pub name: String,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl Item {
    /// Replace the name and refresh the `updated_at` timestamp to now (UTC).
    pub fn rename(&mut self, name: String) {
        self.name = name;
        self.updated_at = OffsetDateTime::now_utc();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rename_updates_name_and_timestamp() {
        let mut item = Item {
            id: Uuid::nil(),
            name: "old".to_string(),
            created_at: OffsetDateTime::UNIX_EPOCH,
            updated_at: OffsetDateTime::UNIX_EPOCH,
        };
        item.rename("new".to_string());
        assert_eq!(item.name, "new");
        assert!(item.updated_at > OffsetDateTime::UNIX_EPOCH);
    }
}
