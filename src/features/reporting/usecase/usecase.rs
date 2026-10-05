//! Reporting use-case implementation (Go `usecase/usecase.go` parity, port-spec
//! §5.4d). Business rules:
//!
//! * absent / `top <= 0` → `default_top_machines`
//! * `top > max_top_machines` → `InvalidTopMachines` **before any repository call**
//! * otherwise `get_overview` then `get_top_machines(top)`, mapped to contract
//!
//! Config values `<= 0` fall back to 5 / 20 (Go fallbacks).

use std::sync::Arc;

use crate::features::reporting::contract::{
    CatalogStats, MachineSales as ContractMachineSales, MachineStats, SalesStats, SummaryRequest,
    SummaryResponse,
};
use crate::features::reporting::domain::{Error, MachineSales, Overview};
use crate::features::reporting::repository::Repository;
use crate::features::reporting::usecase::Service;

const DEFAULT_TOP_MACHINES: i32 = 5;
const MAX_TOP_MACHINES: i32 = 20;

/// Concrete reporting use case backed by a [`Repository`] outbound interface.
#[derive(Clone)]
pub struct Usecase {
    repo: Arc<dyn Repository>,
    default_top_machines: i32,
    max_top_machines: i32,
}

impl Usecase {
    /// Build a use case. Values `<= 0` fall back to 5 / 20.
    pub fn new(
        repo: Arc<dyn Repository>,
        default_top_machines: i32,
        max_top_machines: i32,
    ) -> Self {
        Self {
            repo,
            default_top_machines: positive_or(default_top_machines, DEFAULT_TOP_MACHINES),
            max_top_machines: positive_or(max_top_machines, MAX_TOP_MACHINES),
        }
    }

    fn summary_response(overview: Overview, top: &[MachineSales]) -> SummaryResponse {
        SummaryResponse {
            catalog: CatalogStats {
                product_count: overview.product_count,
                total_stock: overview.total_stock,
            },
            machines: MachineStats {
                machine_count: overview.machine_count,
                total_coin_bank_cents: overview.total_bank_cents,
            },
            sales: SalesStats {
                purchase_count: overview.purchase_count,
                revenue_cents: overview.revenue_cents,
            },
            top_machines: top
                .iter()
                .map(|m| ContractMachineSales {
                    machine_id: m.machine_id.to_string(),
                    label: m.label.clone(),
                    purchase_count: m.purchase_count,
                    revenue_cents: m.revenue_cents,
                })
                .collect(),
        }
    }
}

#[async_trait::async_trait]
impl Service for Usecase {
    async fn summary(&self, req: SummaryRequest) -> Result<SummaryResponse, Error> {
        let requested = req.top.unwrap_or(0);
        let top = if requested <= 0 {
            self.default_top_machines
        } else {
            requested
        };
        if top > self.max_top_machines {
            return Err(Error::InvalidTopMachines);
        }
        let overview = self.repo.get_overview().await?;
        let top_machines = self.repo.get_top_machines(top).await?;
        Ok(Self::summary_response(overview, &top_machines))
    }
}

fn positive_or(value: i32, fallback: i32) -> i32 {
    if value <= 0 { fallback } else { value }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::reporting::repository::MockRepository;
    use mockall::predicate::*;
    use uuid::Uuid;

    fn overview() -> Overview {
        Overview {
            product_count: 2,
            total_stock: 9,
            machine_count: 1,
            total_bank_cents: 250,
            purchase_count: 3,
            revenue_cents: 300,
        }
    }

    fn usecase(repo: MockRepository) -> Usecase {
        Usecase::new(Arc::new(repo), 5, 20)
    }

    #[tokio::test]
    async fn summary_uses_default_when_top_absent_or_zero() {
        let mut mock = MockRepository::new();
        mock.expect_get_overview().returning(|| Ok(overview()));
        mock.expect_get_top_machines()
            .with(eq(5))
            .returning(|_| Ok(vec![]))
            .times(2);
        let svc = usecase(mock);
        svc.summary(SummaryRequest { top: None }).await.unwrap();
        svc.summary(SummaryRequest { top: Some(0) }).await.unwrap();
    }

    #[tokio::test]
    async fn summary_rejects_top_above_max_before_any_repository_call() {
        let mut mock = MockRepository::new();
        mock.expect_get_overview().times(0);
        mock.expect_get_top_machines().times(0);
        let svc = usecase(mock);
        assert_eq!(
            svc.summary(SummaryRequest { top: Some(21) })
                .await
                .unwrap_err(),
            Error::InvalidTopMachines
        );
    }

    #[tokio::test]
    async fn summary_maps_overview_and_top_machines() {
        let mut mock = MockRepository::new();
        mock.expect_get_overview().returning(|| Ok(overview()));
        mock.expect_get_top_machines().with(eq(3)).returning(|_| {
            Ok(vec![MachineSales {
                machine_id: Uuid::nil(),
                label: "lobby".to_string(),
                purchase_count: 4,
                revenue_cents: 500,
            }])
        });
        let svc = usecase(mock);
        let resp = svc.summary(SummaryRequest { top: Some(3) }).await.unwrap();
        assert_eq!(resp.catalog.product_count, 2);
        assert_eq!(resp.catalog.total_stock, 9);
        assert_eq!(resp.machines.machine_count, 1);
        assert_eq!(resp.machines.total_coin_bank_cents, 250);
        assert_eq!(resp.sales.purchase_count, 3);
        assert_eq!(resp.sales.revenue_cents, 300);
        assert_eq!(resp.top_machines.len(), 1);
        assert_eq!(resp.top_machines[0].machine_id, Uuid::nil().to_string());
        assert_eq!(resp.top_machines[0].label, "lobby");
    }

    #[tokio::test]
    async fn summary_accepts_exactly_max() {
        let mut mock = MockRepository::new();
        mock.expect_get_overview().returning(|| Ok(overview()));
        mock.expect_get_top_machines()
            .with(eq(20))
            .returning(|_| Ok(vec![]));
        let svc = usecase(mock);
        svc.summary(SummaryRequest { top: Some(20) }).await.unwrap();
    }

    #[tokio::test]
    async fn fallback_defaults_apply_when_config_zero() {
        let mut mock = MockRepository::new();
        mock.expect_get_overview().returning(|| Ok(overview()));
        mock.expect_get_top_machines()
            .with(eq(5))
            .returning(|_| Ok(vec![]));
        let svc = Usecase::new(Arc::new(mock), 0, 0);
        svc.summary(SummaryRequest { top: None }).await.unwrap();
    }

    #[tokio::test]
    async fn summary_propagates_repository_errors() {
        let mut mock = MockRepository::new();
        mock.expect_get_overview().returning(|| {
            Err(Error::Internal {
                cause: Some(anyhow::anyhow!("boom")),
            })
        });
        let svc = usecase(mock);
        assert!(matches!(
            svc.summary(SummaryRequest { top: None }).await,
            Err(Error::Internal { .. })
        ));
    }
}
