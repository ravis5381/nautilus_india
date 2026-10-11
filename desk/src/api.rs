use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;

use crate::model::{money, CreateOrder, Fill, Order, OrderStatus, Position, Side};
use crate::store::{Desk, DeskError, Submission};

pub type Shared = Arc<Mutex<Desk>>;

pub fn router(desk: Shared) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/app.css", get(css))
        .route("/app.js", get(javascript))
        .route("/api/positions", get(positions))
        .route("/api/orders", get(orders).post(create_order))
        .route("/api/fills", get(fills))
        .with_state(desk)
}

async fn index() -> Response {
    asset(include_str!("../static/index.html"), "text/html; charset=utf-8")
}

async fn css() -> Response {
    asset(include_str!("../static/app.css"), "text/css; charset=utf-8")
}

async fn javascript() -> Response {
    asset(include_str!("../static/app.js"), "text/javascript; charset=utf-8")
}

fn asset(body: &'static str, content_type: &'static str) -> Response {
    (
        [
            (header::CONTENT_TYPE, content_type),
            (header::CACHE_CONTROL, "no-store"),
        ],
        body,
    )
        .into_response()
}

async fn positions(State(desk): State<Shared>) -> Result<Json<Vec<PositionBody>>, ApiError> {
    let desk = lock(&desk)?;
    let rows = desk.positions()?.into_iter().map(PositionBody::from).collect();
    Ok(Json(rows))
}

async fn orders(State(desk): State<Shared>) -> Result<Json<Vec<OrderBody>>, ApiError> {
    let desk = lock(&desk)?;
    let rows = desk.orders()?.into_iter().map(OrderBody::from).collect();
    Ok(Json(rows))
}

async fn fills(State(desk): State<Shared>) -> Result<Json<Vec<FillBody>>, ApiError> {
    let desk = lock(&desk)?;
    let rows = desk.fills()?.into_iter().map(FillBody::from).collect();
    Ok(Json(rows))
}

async fn create_order(
    State(desk): State<Shared>,
    Json(input): Json<CreateOrder>,
) -> Result<(StatusCode, Json<SubmissionBody>), ApiError> {
    let mut desk = lock(&desk)?;
    let submission = desk.submit(input)?;
    Ok((StatusCode::CREATED, Json(SubmissionBody::from(submission))))
}

fn lock(desk: &Shared) -> Result<std::sync::MutexGuard<'_, Desk>, ApiError> {
    desk.lock().map_err(|_| ApiError::storage("The position book lock was poisoned."))
}

#[derive(Serialize)]
struct PositionBody {
    strategy: String,
    instrument_id: String,
    quantity: i64,
    side: Option<Side>,
    avg_price: String,
    realized_pnl: String,
}

impl From<Position> for PositionBody {
    fn from(position: Position) -> Self {
        let side = position.side();
        Self {
            strategy: position.strategy,
            instrument_id: position.instrument_id,
            quantity: position.quantity,
            side,
            avg_price: money(position.avg_price),
            realized_pnl: money(position.realized_pnl),
        }
    }
}

#[derive(Serialize)]
struct OrderBody {
    id: String,
    strategy: String,
    instrument_id: String,
    side: Side,
    quantity: i64,
    price: String,
    status: OrderStatus,
    gateway: String,
    reason: Option<String>,
    created_at: String,
}

impl From<Order> for OrderBody {
    fn from(order: Order) -> Self {
        Self {
            id: order.id,
            strategy: order.strategy,
            instrument_id: order.instrument_id,
            side: order.side,
            quantity: order.quantity,
            price: money(order.price),
            status: order.status,
            gateway: order.gateway,
            reason: order.reason,
            created_at: order.created_at.to_rfc3339(),
        }
    }
}

#[derive(Serialize)]
struct FillBody {
    id: String,
    order_id: String,
    strategy: String,
    instrument_id: String,
    side: Side,
    quantity: i64,
    price: String,
    filled_at: String,
}

impl From<Fill> for FillBody {
    fn from(fill: Fill) -> Self {
        Self {
            id: fill.id,
            order_id: fill.order_id,
            strategy: fill.strategy,
            instrument_id: fill.instrument_id,
            side: fill.side,
            quantity: fill.quantity,
            price: money(fill.price),
            filled_at: fill.filled_at.to_rfc3339(),
        }
    }
}

#[derive(Serialize)]
struct SubmissionBody {
    order: OrderBody,
    fill: Option<FillBody>,
    position: Option<PositionBody>,
}

impl From<Submission> for SubmissionBody {
    fn from(submission: Submission) -> Self {
        Self {
            order: submission.order.into(),
            fill: submission.fill.map(FillBody::from),
            position: submission.position.map(PositionBody::from),
        }
    }
}

struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn storage(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: message.into(),
        }
    }
}

impl From<DeskError> for ApiError {
    fn from(error: DeskError) -> Self {
        let status = match error {
            DeskError::Invalid(_) | DeskError::Risk(_) => StatusCode::UNPROCESSABLE_ENTITY,
            DeskError::Broker(_) => StatusCode::BAD_GATEWAY,
            DeskError::Storage(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        Self {
            status,
            message: error.to_string(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = Json(serde_json::json!({ "error": self.message }));
        (self.status, body).into_response()
    }
}
