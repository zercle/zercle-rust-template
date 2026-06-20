//! Health registry (liveness + readiness checkers). Mirrors structure.md §8 / Go `health.go`.

use std::sync::Arc;

use anyhow::{Result, anyhow, bail};
use futures::future::join_all;

/// A health probe for a single dependency.
#[async_trait::async_trait]
pub trait Checker: Send + Sync {
    fn name(&self) -> &'static str;
    async fn check(&self) -> Result<()>;
}

#[derive(Default)]
pub struct Registry {
    liveness: Vec<Arc<dyn Checker>>,
    readiness: Vec<Arc<dyn Checker>>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_liveness(&mut self, c: Arc<dyn Checker>) {
        self.liveness.push(c);
    }

    pub fn add_readiness(&mut self, c: Arc<dyn Checker>) {
        self.readiness.push(c);
    }

    pub async fn live(&self) -> Result<()> {
        run(&self.liveness).await
    }

    pub async fn ready(&self) -> Result<()> {
        run(&self.readiness).await
    }
}

async fn run(checkers: &[Arc<dyn Checker>]) -> Result<()> {
    if checkers.is_empty() {
        return Ok(());
    }
    let results = join_all(
        checkers
            .iter()
            .map(|c| async move { (c.name(), c.check().await) }),
    )
    .await;

    let failures: Vec<String> = results
        .into_iter()
        .filter_map(|(name, r)| r.err().map(|e| format!("{name}: {e:#}")))
        .collect();

    if failures.is_empty() {
        Ok(())
    } else {
        bail!("{}", failures.join("; "))
    }
}

/// Convenience wrapper used by readiness handlers.
pub async fn ready(reg: &Registry) -> Result<()> {
    reg.ready().await.map_err(|e| anyhow!(e))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct OkChecker;
    #[async_trait::async_trait]
    impl Checker for OkChecker {
        fn name(&self) -> &'static str {
            "ok"
        }
        async fn check(&self) -> Result<()> {
            Ok(())
        }
    }

    struct BadChecker;
    #[async_trait::async_trait]
    impl Checker for BadChecker {
        fn name(&self) -> &'static str {
            "bad"
        }
        async fn check(&self) -> Result<()> {
            anyhow::bail!("nope")
        }
    }

    #[tokio::test]
    async fn empty_registry_is_ok() {
        let r = Registry::new();
        assert!(r.live().await.is_ok());
        assert!(r.ready().await.is_ok());
    }

    #[tokio::test]
    async fn passing_checkers_succeed() {
        let mut r = Registry::new();
        r.add_readiness(Arc::new(OkChecker));
        assert!(r.ready().await.is_ok());
    }

    #[tokio::test]
    async fn failing_checker_name_surfaces() {
        let mut r = Registry::new();
        r.add_readiness(Arc::new(BadChecker));
        let err = r.ready().await.unwrap_err();
        assert!(err.to_string().contains("bad"), "got {err}");
    }

    #[tokio::test]
    async fn mixed_run_concurrently() {
        let mut r = Registry::new();
        r.add_readiness(Arc::new(OkChecker));
        r.add_readiness(Arc::new(BadChecker));
        let err = r.ready().await.unwrap_err();
        assert!(err.to_string().contains("bad"));
    }
}
