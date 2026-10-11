use crate::model::{OrderRequest, Position};

/// A check that runs after the order is valid and before it is forwarded.
///
/// The default pipeline has no checks. Exposure, loss, and margin limits
/// belong here so a rejected order never reaches the broker.
pub trait RiskCheck: Send {
    fn check(&self, order: &OrderRequest, positions: &[Position]) -> Result<(), String>;
}

pub struct RiskPipeline {
    checks: Vec<Box<dyn RiskCheck>>,
}

impl RiskPipeline {
    pub fn new(checks: Vec<Box<dyn RiskCheck>>) -> Self {
        Self { checks }
    }

    pub fn check(&self, order: &OrderRequest, positions: &[Position]) -> Result<(), String> {
        for check in &self.checks {
            check.check(order, positions)?;
        }
        Ok(())
    }
}

impl Default for RiskPipeline {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}
