//! Machines use-case implementation (Go `usecase/usecase.go` parity, port-spec
//! §5.2d). Business rules:
//!
//! * label trimmed; empty or `> max_label_length` runes → `InvalidMachineLabel`
//! * non-empty `initial_coins` are validated against the supported denominations
//! * `restock_bank` validates coins, calls the port, then re-reads the machine
//!   so the returned bank is authoritative
//! * pagination clamp same as catalog
//!
//! Config values `<= 0` fall back to 20 / 100 / 255 (Go fallbacks).

use std::sync::Arc;

use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

use crate::features::machines::contract::{
    CreateMachineRequest, ListMachinesRequest, ListMachinesResponse, MachineResponse,
    RestockBankRequest,
};
use crate::features::machines::domain::{Error, Machine, add_coins, validate_coins};
use crate::features::machines::repository::Repository;
use crate::features::machines::usecase::Service;

const DEFAULT_PAGE_SIZE: i32 = 20;
const MAX_PAGE_SIZE: i32 = 100;
const MAX_LABEL_LENGTH: usize = 255;

/// Concrete machines use case backed by a [`Repository`] outbound interface.
#[derive(Clone)]
pub struct Usecase {
    repo: Arc<dyn Repository>,
    default_page_size: i32,
    max_page_size: i32,
    max_label_length: usize,
}

impl Usecase {
    /// Build a use case. Values `<= 0` fall back to 20 / 100 / 255.
    pub fn new(
        repo: Arc<dyn Repository>,
        default_page_size: i32,
        max_page_size: i32,
        max_label_length: i32,
    ) -> Self {
        Self {
            repo,
            default_page_size: positive_or(default_page_size, DEFAULT_PAGE_SIZE),
            max_page_size: positive_or(max_page_size, MAX_PAGE_SIZE),
            max_label_length: positive_or(max_label_length, MAX_LABEL_LENGTH as i32) as usize,
        }
    }

    /// Map a domain machine to its wire form. A nil/empty bank becomes an empty
    /// object, never null (Go `newMachineResponse`).
    fn machine_response(machine: &Machine) -> MachineResponse {
        MachineResponse {
            id: machine.id.to_string(),
            label: machine.label.clone(),
            coin_bank: machine.coin_bank.clone(),
            created_at: format_rfc3339(machine.created_at),
            updated_at: format_rfc3339(machine.updated_at),
        }
    }
}

#[async_trait::async_trait]
impl Service for Usecase {
    async fn create(&self, req: CreateMachineRequest) -> Result<MachineResponse, Error> {
        let label = req.label.trim();
        if label.is_empty() || label.chars().count() > self.max_label_length {
            return Err(Error::InvalidMachineLabel);
        }
        if !req.initial_coins.is_empty() {
            validate_coins(&req.initial_coins)?;
        }
        let now = OffsetDateTime::now_utc();
        let machine = Machine {
            id: Uuid::now_v7(),
            label: label.to_string(),
            coin_bank: add_coins(&Default::default(), &req.initial_coins),
            created_at: now,
            updated_at: now,
        };
        self.repo.create(&machine).await?;
        Ok(Self::machine_response(&machine))
    }

    async fn get(&self, id: String) -> Result<MachineResponse, Error> {
        let id = Uuid::parse_str(&id).map_err(|_| Error::InvalidId)?;
        let machine = self.repo.get_by_id(id).await?;
        Ok(Self::machine_response(&machine))
    }

    async fn list(&self, req: ListMachinesRequest) -> Result<ListMachinesResponse, Error> {
        let mut limit = req.limit.unwrap_or(0);
        if limit <= 0 {
            limit = self.default_page_size;
        }
        if limit > self.max_page_size {
            limit = self.max_page_size;
        }
        let offset = req.offset.unwrap_or(0).max(0);
        let machines = self.repo.list(limit, offset).await?;
        Ok(ListMachinesResponse {
            machines: machines.iter().map(Self::machine_response).collect(),
        })
    }

    async fn restock_bank(
        &self,
        id: String,
        req: RestockBankRequest,
    ) -> Result<MachineResponse, Error> {
        let id = Uuid::parse_str(&id).map_err(|_| Error::InvalidId)?;
        validate_coins(&req.coins)?;
        self.repo.restock_bank(id, &req.coins).await?;
        // Re-read for the authoritative bank after the locked update.
        let machine = self.repo.get_by_id(id).await?;
        Ok(Self::machine_response(&machine))
    }
}

fn positive_or(value: i32, fallback: i32) -> i32 {
    if value <= 0 { fallback } else { value }
}

fn format_rfc3339(t: OffsetDateTime) -> String {
    t.format(&Rfc3339).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::machines::domain::CoinBank;
    use crate::features::machines::repository::MockRepository;
    use mockall::predicate::*;

    fn machine(id: Uuid, label: &str, bank: CoinBank) -> Machine {
        let now = OffsetDateTime::now_utc();
        Machine {
            id,
            label: label.to_string(),
            coin_bank: bank,
            created_at: now,
            updated_at: now,
        }
    }

    fn usecase(repo: MockRepository) -> Usecase {
        Usecase::new(Arc::new(repo), 20, 100, 255)
    }

    #[tokio::test]
    async fn create_rejects_empty_label() {
        let svc = usecase(MockRepository::new());
        assert_eq!(
            svc.create(CreateMachineRequest {
                label: "  ".to_string(),
                initial_coins: vec![],
            })
            .await
            .unwrap_err(),
            Error::InvalidMachineLabel
        );
    }

    #[tokio::test]
    async fn create_rejects_overlong_label() {
        let svc = usecase(MockRepository::new());
        assert_eq!(
            svc.create(CreateMachineRequest {
                label: "a".repeat(256),
                initial_coins: vec![],
            })
            .await
            .unwrap_err(),
            Error::InvalidMachineLabel
        );
    }

    #[tokio::test]
    async fn create_rejects_unsupported_initial_coin() {
        let svc = usecase(MockRepository::new());
        assert_eq!(
            svc.create(CreateMachineRequest {
                label: "lobby".to_string(),
                initial_coins: vec![7],
            })
            .await
            .unwrap_err(),
            Error::UnsupportedCoin
        );
    }

    #[tokio::test]
    async fn create_builds_bank_from_initial_coins() {
        let mut mock = MockRepository::new();
        mock.expect_create().returning(|_| Ok(()));
        let svc = usecase(mock);
        let resp = svc
            .create(CreateMachineRequest {
                label: " lobby ".to_string(),
                initial_coins: vec![25, 25, 100],
            })
            .await
            .unwrap();
        assert_eq!(resp.label, "lobby");
        assert_eq!(resp.coin_bank.get(&25), Some(&2));
        assert_eq!(resp.coin_bank.get(&100), Some(&1));
    }

    #[tokio::test]
    async fn get_rejects_malformed_id_before_touching_the_port() {
        let mut mock = MockRepository::new();
        mock.expect_get_by_id().times(0);
        let svc = usecase(mock);
        assert_eq!(
            svc.get("not-a-uuid".to_string()).await.unwrap_err(),
            Error::InvalidId
        );
    }

    #[tokio::test]
    async fn get_maps_empty_bank_to_empty_object() {
        let mut mock = MockRepository::new();
        mock.expect_get_by_id()
            .with(eq(Uuid::nil()))
            .returning(|id| Ok(machine(id, "lobby", CoinBank::new())));
        let svc = usecase(mock);
        let resp = svc.get(Uuid::nil().to_string()).await.unwrap();
        assert!(resp.coin_bank.is_empty());
    }

    #[tokio::test]
    async fn get_passes_through_not_found() {
        let mut mock = MockRepository::new();
        mock.expect_get_by_id()
            .returning(|_| Err(Error::MachineNotFound));
        let svc = usecase(mock);
        assert_eq!(
            svc.get(Uuid::nil().to_string()).await.unwrap_err(),
            Error::MachineNotFound
        );
    }

    #[tokio::test]
    async fn list_clamps_limit_above_max() {
        let mut mock = MockRepository::new();
        mock.expect_list()
            .withf(|limit, offset| *limit == 100 && *offset == 0)
            .returning(|_, _| Ok(vec![]));
        let svc = usecase(mock);
        let resp = svc
            .list(ListMachinesRequest {
                limit: Some(9_999),
                offset: Some(0),
            })
            .await
            .unwrap();
        assert!(resp.machines.is_empty());
    }

    #[tokio::test]
    async fn list_uses_default_and_clamps_negative_offset() {
        let mut mock = MockRepository::new();
        mock.expect_list()
            .withf(|limit, offset| *limit == 20 && *offset == 0)
            .returning(|_, _| Ok(vec![]))
            .times(2);
        let svc = usecase(mock);
        svc.list(ListMachinesRequest {
            limit: None,
            offset: None,
        })
        .await
        .unwrap();
        svc.list(ListMachinesRequest {
            limit: Some(0),
            offset: Some(-9),
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn fallback_defaults_apply_when_config_zero() {
        let mut mock = MockRepository::new();
        mock.expect_list()
            .withf(|limit, offset| *limit == 100 && *offset == 0)
            .returning(|_, _| Ok(vec![]));
        let svc = Usecase::new(Arc::new(mock), 0, 0, 0);
        svc.list(ListMachinesRequest {
            limit: Some(9_999),
            offset: Some(0),
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn restock_rejects_unsupported_coin_before_port() {
        let mut mock = MockRepository::new();
        mock.expect_restock_bank().times(0);
        let svc = usecase(mock);
        assert_eq!(
            svc.restock_bank(
                Uuid::nil().to_string(),
                RestockBankRequest { coins: vec![7] },
            )
            .await
            .unwrap_err(),
            Error::UnsupportedCoin
        );
    }

    #[tokio::test]
    async fn restock_rejects_malformed_id_before_port() {
        let mut mock = MockRepository::new();
        mock.expect_restock_bank().times(0);
        let svc = usecase(mock);
        assert_eq!(
            svc.restock_bank(
                "not-a-uuid".to_string(),
                RestockBankRequest { coins: vec![25] },
            )
            .await
            .unwrap_err(),
            Error::InvalidId
        );
    }

    #[tokio::test]
    async fn restock_re_reads_the_authoritative_bank() {
        let mut mock = MockRepository::new();
        mock.expect_restock_bank()
            .with(eq(Uuid::nil()), eq(vec![25]))
            .returning(|_, _| Ok(()));
        let mut bank = CoinBank::new();
        bank.insert(25, 3);
        mock.expect_get_by_id()
            .with(eq(Uuid::nil()))
            .returning(move |id| Ok(machine(id, "lobby", bank.clone())));
        let svc = usecase(mock);
        let resp = svc
            .restock_bank(
                Uuid::nil().to_string(),
                RestockBankRequest { coins: vec![25] },
            )
            .await
            .unwrap();
        assert_eq!(resp.coin_bank.get(&25), Some(&3));
    }
}
