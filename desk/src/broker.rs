use chrono::Utc;
use uuid::Uuid;

use crate::model::{Fill, Order};

/// Forwards an accepted order to a broker and returns the fill when the
/// gateway already knows the execution.
pub trait BrokerGateway: Send {
    fn name(&self) -> &str;
    fn submit(&mut self, order: &Order) -> Result<Fill, String>;
}

/// Fills the order at its submitted price. A live broker adapter replaces this
/// and can ack the order before a fill arrives.
pub struct PaperBroker;

impl BrokerGateway for PaperBroker {
    fn name(&self) -> &str {
        "paper"
    }

    fn submit(&mut self, order: &Order) -> Result<Fill, String> {
        Ok(Fill {
            id: Uuid::new_v4().to_string(),
            order_id: order.id.clone(),
            strategy: order.strategy.clone(),
            instrument_id: order.instrument_id.clone(),
            side: order.side,
            quantity: order.quantity,
            price: order.price,
            filled_at: Utc::now(),
        })
    }
}
