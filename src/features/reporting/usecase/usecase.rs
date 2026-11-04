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
mod tests;
