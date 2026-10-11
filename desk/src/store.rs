use std::path::Path;
use std::str::FromStr;

use chrono::Utc;
use rusqlite::{params, Connection};
use rust_decimal::Decimal;

#[cfg(test)]
use chrono::DateTime;
#[cfg(test)]
use uuid::Uuid;

#[cfg(test)]
use crate::broker::{BrokerGateway, PaperBroker};
use crate::model::{money, Position};
#[cfg(test)]
use crate::model::{CreateOrder, Fill, Order, OrderStatus, Side};
#[cfg(test)]
use crate::position::apply_fill;
#[cfg(test)]
use crate::risk::RiskPipeline;

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS orders (
    id TEXT PRIMARY KEY,
    strategy TEXT NOT NULL,
    instrument_id TEXT NOT NULL,
    side TEXT NOT NULL,
    quantity INTEGER NOT NULL,
    price TEXT NOT NULL,
    status TEXT NOT NULL,
    gateway TEXT NOT NULL,
    reason TEXT,
    created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS fills (
    id TEXT PRIMARY KEY,
    order_id TEXT NOT NULL,
    strategy TEXT NOT NULL,
    instrument_id TEXT NOT NULL,
    side TEXT NOT NULL,
    quantity INTEGER NOT NULL,
    price TEXT NOT NULL,
    filled_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS positions (
    strategy TEXT NOT NULL,
    instrument_id TEXT NOT NULL,
    quantity INTEGER NOT NULL,
    avg_price TEXT NOT NULL,
    realized_pnl TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (strategy, instrument_id)
);
";

pub struct Desk {
    conn: Connection,
    #[cfg(test)]
    risk: RiskPipeline,
    #[cfg(test)]
    broker: Box<dyn BrokerGateway>,
}

#[cfg(test)]
#[derive(Debug)]
pub struct Submission {
    pub order: Order,
    #[allow(dead_code)]
    pub fill: Option<Fill>,
    pub position: Option<Position>,
}

impl Desk {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, DeskError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        Self::connect(Connection::open(path)?)
    }

    #[cfg(test)]
    pub fn open_in_memory() -> Result<Self, DeskError> {
        Self::from_conn(
            Connection::open_in_memory()?,
            RiskPipeline::default(),
            Box::new(PaperBroker),
        )
    }

    fn connect(conn: Connection) -> Result<Self, DeskError> {
        conn.execute_batch(SCHEMA)?;
        Ok(Self {
            conn,
            #[cfg(test)]
            risk: RiskPipeline::default(),
            #[cfg(test)]
            broker: Box::new(PaperBroker),
        })
    }

    #[cfg(test)]
    pub fn from_conn(
        conn: Connection,
        risk: RiskPipeline,
        broker: Box<dyn BrokerGateway>,
    ) -> Result<Self, DeskError> {
        conn.execute_batch(SCHEMA)?;
        Ok(Self { conn, risk, broker })
    }

    #[cfg(test)]
    pub fn submit(&mut self, input: CreateOrder) -> Result<Submission, DeskError> {
        let request = input.parse().map_err(DeskError::Invalid)?;
        let now = Utc::now();
        let gateway = self.broker.name().to_string();
        let mut order = Order {
            id: Uuid::new_v4().to_string(),
            strategy: request.strategy.clone(),
            instrument_id: request.instrument_id.clone(),
            side: request.side,
            quantity: request.quantity,
            price: request.price,
            status: OrderStatus::Rejected,
            gateway,
            reason: None,
            created_at: now,
        };

        let positions = self.positions()?;
        if let Err(reason) = self.risk.check(&request, &positions) {
            order.reason = Some(reason.clone());
            self.insert_order(&order)?;
            return Err(DeskError::Risk(reason));
        }

        let fill = match self.broker.submit(&order) {
            Ok(fill) => fill,
            Err(reason) => {
                order.reason = Some(reason.clone());
                self.insert_order(&order)?;
                return Err(DeskError::Broker(reason));
            }
        };

        order.status = OrderStatus::Filled;
        let position = self.apply_and_store(&order, &fill)?;
        Ok(Submission {
            order,
            fill: Some(fill),
            position: Some(position),
        })
    }

    pub fn positions(&self) -> Result<Vec<Position>, DeskError> {
        self.query_positions(None)
    }

    pub fn positions_for_strategy(&self, strategy: &str) -> Result<Vec<Position>, DeskError> {
        self.query_positions(Some(strategy))
    }

    pub fn set_positions(&mut self, positions: Vec<Position>) -> Result<Vec<Position>, DeskError> {
        let tx = self.conn.transaction()?;
        let updated_at = Utc::now().to_rfc3339();
        for position in &positions {
            tx.execute(
                "INSERT INTO positions (strategy, instrument_id, quantity, avg_price, realized_pnl, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(strategy, instrument_id) DO UPDATE SET
                    quantity = excluded.quantity,
                    avg_price = excluded.avg_price,
                    realized_pnl = excluded.realized_pnl,
                    updated_at = excluded.updated_at",
                params![
                    position.strategy,
                    position.instrument_id,
                    position.quantity,
                    money(position.avg_price),
                    money(position.realized_pnl),
                    updated_at,
                ],
            )?;
        }
        tx.commit()?;
        Ok(positions)
    }

    fn query_positions(&self, strategy: Option<&str>) -> Result<Vec<Position>, DeskError> {
        let mut statement = self.conn.prepare(
            "SELECT strategy, instrument_id, quantity, avg_price, realized_pnl
             FROM positions
             WHERE (?1 IS NULL OR strategy = ?1)
             ORDER BY strategy, instrument_id",
        )?;
        let rows = statement.query_map(params![strategy], |row| {
            Ok(Position {
                strategy: row.get(0)?,
                instrument_id: row.get(1)?,
                quantity: row.get(2)?,
                avg_price: parse_decimal(&row.get::<_, String>(3)?),
                realized_pnl: parse_decimal(&row.get::<_, String>(4)?),
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DeskError::from)
    }

    #[cfg(test)]
    pub fn orders(&self) -> Result<Vec<Order>, DeskError> {
        let mut statement = self.conn.prepare(
            "SELECT id, strategy, instrument_id, side, quantity, price, status, gateway, reason, created_at
             FROM orders
             ORDER BY created_at DESC",
        )?;
        let rows = statement.query_map([], |row| {
            let side: String = row.get(3)?;
            let status: String = row.get(6)?;
            let created_at: String = row.get(9)?;
            Ok(Order {
                id: row.get(0)?,
                strategy: row.get(1)?,
                instrument_id: row.get(2)?,
                side: parse_side(&side),
                quantity: row.get(4)?,
                price: parse_decimal(&row.get::<_, String>(5)?),
                status: parse_status(&status),
                gateway: row.get(7)?,
                reason: row.get(8)?,
                created_at: DateTime::parse_from_rfc3339(&created_at)
                    .map(|value| value.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DeskError::from)
    }

    #[cfg(test)]
    pub fn fills(&self) -> Result<Vec<Fill>, DeskError> {
        let mut statement = self.conn.prepare(
            "SELECT id, order_id, strategy, instrument_id, side, quantity, price, filled_at
             FROM fills
             ORDER BY filled_at DESC",
        )?;
        let rows = statement.query_map([], |row| {
            let side: String = row.get(4)?;
            let filled_at: String = row.get(7)?;
            Ok(Fill {
                id: row.get(0)?,
                order_id: row.get(1)?,
                strategy: row.get(2)?,
                instrument_id: row.get(3)?,
                side: parse_side(&side),
                quantity: row.get(5)?,
                price: parse_decimal(&row.get::<_, String>(6)?),
                filled_at: DateTime::parse_from_rfc3339(&filled_at)
                    .map(|value| value.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DeskError::from)
    }

    #[cfg(test)]
    fn apply_and_store(&mut self, order: &Order, fill: &Fill) -> Result<Position, DeskError> {
        let tx = self.conn.transaction()?;
        insert_order(&tx, order)?;
        tx.execute(
            "INSERT INTO fills (id, order_id, strategy, instrument_id, side, quantity, price, filled_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                fill.id,
                fill.order_id,
                fill.strategy,
                fill.instrument_id,
                fill.side.to_string(),
                fill.quantity,
                money(fill.price),
                fill.filled_at.to_rfc3339(),
            ],
        )?;

        let mut position = load_position(&tx, &fill.strategy, &fill.instrument_id)?;
        apply_fill(&mut position, fill.side, fill.quantity, fill.price);
        tx.execute(
            "INSERT INTO positions (strategy, instrument_id, quantity, avg_price, realized_pnl, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(strategy, instrument_id) DO UPDATE SET
                quantity = excluded.quantity,
                avg_price = excluded.avg_price,
                realized_pnl = excluded.realized_pnl,
                updated_at = excluded.updated_at",
            params![
                position.strategy,
                position.instrument_id,
                position.quantity,
                money(position.avg_price),
                money(position.realized_pnl),
                Utc::now().to_rfc3339(),
            ],
        )?;
        tx.commit()?;
        Ok(position)
    }

    #[cfg(test)]
    fn insert_order(&mut self, order: &Order) -> Result<(), DeskError> {
        insert_order(&self.conn, order)
    }
}

#[cfg(test)]
fn insert_order(conn: &Connection, order: &Order) -> Result<(), DeskError> {
    conn.execute(
        "INSERT INTO orders
            (id, strategy, instrument_id, side, quantity, price, status, gateway, reason, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            order.id,
            order.strategy,
            order.instrument_id,
            order.side.to_string(),
            order.quantity,
            money(order.price),
            order.status.to_string(),
            order.gateway,
            order.reason,
            order.created_at.to_rfc3339(),
        ],
    )?;
    Ok(())
}

#[cfg(test)]
fn load_position(
    conn: &Connection,
    strategy: &str,
    instrument_id: &str,
) -> Result<Position, DeskError> {
    let mut statement = conn.prepare(
        "SELECT quantity, avg_price, realized_pnl
         FROM positions
         WHERE strategy = ?1 AND instrument_id = ?2",
    )?;
    let mut rows = statement.query(params![strategy, instrument_id])?;
    if let Some(row) = rows.next()? {
        return Ok(Position {
            strategy: strategy.to_string(),
            instrument_id: instrument_id.to_string(),
            quantity: row.get(0)?,
            avg_price: parse_decimal(&row.get::<_, String>(1)?),
            realized_pnl: parse_decimal(&row.get::<_, String>(2)?),
        });
    }
    Ok(Position::new(strategy, instrument_id))
}

fn parse_decimal(value: &str) -> Decimal {
    Decimal::from_str(value).unwrap_or(Decimal::ZERO)
}

#[cfg(test)]
fn parse_side(value: &str) -> Side {
    if value == "SELL" {
        Side::Sell
    } else {
        Side::Buy
    }
}

#[cfg(test)]
fn parse_status(value: &str) -> OrderStatus {
    if value == "FILLED" {
        OrderStatus::Filled
    } else {
        OrderStatus::Rejected
    }
}

#[derive(Debug)]
pub enum DeskError {
    Invalid(String),
    Risk(String),
    Broker(String),
    Storage(String),
}

impl std::fmt::Display for DeskError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(message)
            | Self::Risk(message)
            | Self::Broker(message)
            | Self::Storage(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for DeskError {}

impl From<rusqlite::Error> for DeskError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Storage(error.to_string())
    }
}

impl From<std::io::Error> for DeskError {
    fn from(error: std::io::Error) -> Self {
        Self::Storage(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::OrderRequest;
    use crate::risk::RiskCheck;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    struct RejectExposure;

    impl RiskCheck for RejectExposure {
        fn check(&self, order: &OrderRequest, _positions: &[Position]) -> Result<(), String> {
            if order.quantity > 5 {
                Err("Order quantity is above the desk limit of 5.".into())
            } else {
                Ok(())
            }
        }
    }

    struct SpyBroker {
        called: Arc<AtomicBool>,
    }

    impl BrokerGateway for SpyBroker {
        fn name(&self) -> &str {
            "spy"
        }

        fn submit(&mut self, order: &Order) -> Result<Fill, String> {
            self.called.store(true, Ordering::SeqCst);
            BrokerGateway::submit(&mut PaperBroker, order)
        }
    }

    fn order(instrument_id: &str, side: Side, quantity: i64, price: &str) -> CreateOrder {
        order_for("core", instrument_id, side, quantity, price)
    }

    fn order_for(
        strategy: &str,
        instrument_id: &str,
        side: Side,
        quantity: i64,
        price: &str,
    ) -> CreateOrder {
        CreateOrder {
            strategy: strategy.into(),
            instrument_id: instrument_id.into(),
            side,
            quantity,
            price: price.into(),
        }
    }

    #[test]
    fn paper_fills_update_the_book_and_share_one_canonical_id() {
        let mut desk = Desk::open_in_memory().unwrap();
        desk.submit(order("reliance.nse", Side::Buy, 10, "2450.05"))
            .unwrap();
        let second = desk
            .submit(order("RELIANCE.NSE", Side::Sell, 4, "2460"))
            .unwrap();

        let position = second.position.unwrap();
        assert_eq!(position.instrument_id, "RELIANCE.NSE");
        assert_eq!(position.quantity, 6);
        assert_eq!(desk.positions().unwrap().len(), 1);
        assert_eq!(desk.orders().unwrap().len(), 2);
        assert_eq!(desk.fills().unwrap().len(), 2);
        assert_eq!(second.order.status, OrderStatus::Filled);
        assert_eq!(second.order.gateway, "paper");
        assert_eq!(second.order.strategy, "core");
    }

    #[test]
    fn the_same_instrument_keeps_a_separate_book_per_strategy() {
        let mut desk = Desk::open_in_memory().unwrap();
        desk.submit(order_for(
            "opening-drive",
            "RELIANCE.NSE",
            Side::Buy,
            10,
            "2450",
        ))
        .unwrap();
        desk.submit(order_for("hedge", "RELIANCE.NSE", Side::Sell, 4, "2460"))
            .unwrap();
        desk.submit(order_for(
            "opening-drive",
            "RELIANCE.NSE",
            Side::Sell,
            3,
            "2465",
        ))
        .unwrap();

        let positions = desk.positions().unwrap();
        let opening = positions
            .iter()
            .find(|position| position.strategy == "opening-drive")
            .unwrap();
        let hedge = positions
            .iter()
            .find(|position| position.strategy == "hedge")
            .unwrap();
        assert_eq!(opening.quantity, 7);
        assert_eq!(hedge.quantity, -4);
    }

    #[test]
    fn hyphenated_option_ids_stay_on_one_position() {
        let mut desk = Desk::open_in_memory().unwrap();
        desk.submit(order(
            "bajaj-auto-20260625-8500-ce.nfo",
            Side::Buy,
            65,
            "120.5",
        ))
        .unwrap();
        let position = desk.positions().unwrap().remove(0);
        assert_eq!(position.instrument_id, "BAJAJ-AUTO-20260625-8500-CE.NFO");
        assert_eq!(position.quantity, 65);
    }

    #[test]
    fn risk_rejection_is_recorded_and_not_forwarded() {
        let called = Arc::new(AtomicBool::new(false));
        let conn = Connection::open_in_memory().unwrap();
        let mut desk = Desk::from_conn(
            conn,
            RiskPipeline::new(vec![Box::new(RejectExposure)]),
            Box::new(SpyBroker {
                called: called.clone(),
            }),
        )
        .unwrap();

        let error = desk
            .submit(order("RELIANCE.NSE", Side::Buy, 10, "100"))
            .unwrap_err();
        assert!(matches!(error, DeskError::Risk(_)));
        assert!(!called.load(Ordering::SeqCst));
        let orders = desk.orders().unwrap();
        assert_eq!(orders.len(), 1);
        assert_eq!(orders[0].status, OrderStatus::Rejected);
        assert!(desk.positions().unwrap().is_empty());
    }

    #[test]
    fn impossible_expiry_never_reaches_the_gateway() {
        let mut desk = Desk::open_in_memory().unwrap();
        let error = desk
            .submit(order("NIFTY-20260231-FUT.NFO", Side::Buy, 1, "100"))
            .unwrap_err();
        assert!(matches!(error, DeskError::Invalid(_)));
        assert!(desk.orders().unwrap().is_empty());
    }

    #[test]
    fn set_positions_writes_one_strategy_without_changing_another() {
        let mut desk = Desk::open_in_memory().unwrap();
        desk.set_positions(vec![
            set_row("opening-drive", "reliance.nse", 10, "2450", "0"),
            set_row("hedge", "RELIANCE.NSE", -4, "2460", "12.5"),
        ])
        .unwrap();
        desk.set_positions(vec![set_row(
            "opening-drive",
            "RELIANCE.NSE",
            7,
            "2455",
            "40",
        )])
        .unwrap();

        let opening = desk.positions_for_strategy("opening-drive").unwrap();
        assert_eq!(opening.len(), 1);
        assert_eq!(opening[0].instrument_id, "RELIANCE.NSE");
        assert_eq!(opening[0].quantity, 7);
        let hedge = desk.positions_for_strategy("hedge").unwrap();
        assert_eq!(hedge.len(), 1);
        assert_eq!(hedge[0].quantity, -4);
    }

    fn set_row(
        strategy: &str,
        instrument_id: &str,
        quantity: i64,
        avg_price: &str,
        realized_pnl: &str,
    ) -> Position {
        crate::model::SetPosition {
            strategy: strategy.into(),
            instrument_id: instrument_id.into(),
            quantity,
            avg_price: avg_price.into(),
            realized_pnl: realized_pnl.into(),
        }
        .parse()
        .unwrap()
    }
}
