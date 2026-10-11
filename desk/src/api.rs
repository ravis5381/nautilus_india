use std::sync::{Arc, Mutex};

use axum::extract::{Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;

use crate::model::{
    money, normalize_strategy_name, CreateOrder, Fill, Order, OrderStatus, Position, SetPosition,
    Side,
};
use crate::store::{Desk, DeskError, Submission};

pub type Shared = Arc<Mutex<Desk>>;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "India desk",
        version = "0.1.0",
        description = "Position book and order gateway. Each strategy keeps its own quantity for an instrument. The book changes when the gateway reports a fill."
    ),
    paths(positions, set_positions, orders, create_order, fills),
    components(schemas(
        PositionBody,
        SetPosition,
        OrderBody,
        FillBody,
        SubmissionBody,
        CreateOrder,
        ErrorBody,
        Side,
        OrderStatus
    )),
    tags(
        (name = "positions", description = "Strategy books"),
        (name = "orders", description = "Orders forwarded to the gateway"),
        (name = "fills", description = "Executions reported by the gateway")
    )
)]
struct ApiDoc;

pub fn router(desk: Shared) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/app.css", get(css))
        .route("/app.js", get(javascript))
        .route("/api/positions", get(positions).put(set_positions))
        .route("/api/orders", get(orders).post(create_order))
        .route("/api/fills", get(fills))
        .with_state(desk)
        .merge(SwaggerUi::new("/docs").url("/api/openapi.json", ApiDoc::openapi()))
}

async fn index() -> Response {
    asset(
        include_str!("../static/index.html"),
        "text/html; charset=utf-8",
    )
}

async fn css() -> Response {
    asset(include_str!("../static/app.css"), "text/css; charset=utf-8")
}

async fn javascript() -> Response {
    asset(
        include_str!("../static/app.js"),
        "text/javascript; charset=utf-8",
    )
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

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
struct PositionQuery {
    /// Strategy identifier. Omit it to return every strategy book.
    strategy: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/positions",
    tag = "positions",
    params(PositionQuery),
    responses(
        (status = 200, description = "Positions for one strategy, or every strategy when the identifier is omitted", body = Vec<PositionBody>),
        (status = 422, description = "The strategy identifier is not valid", body = ErrorBody),
        (status = 500, description = "The book could not be read", body = ErrorBody)
    )
)]
async fn positions(
    State(desk): State<Shared>,
    Query(query): Query<PositionQuery>,
) -> Result<Json<Vec<PositionBody>>, ApiError> {
    let desk = lock(&desk)?;
    let rows = match query.strategy {
        Some(strategy) => {
            let strategy = normalize_strategy_name(&strategy).map_err(DeskError::Invalid)?;
            desk.positions_for_strategy(&strategy)?
        }
        None => desk.positions()?,
    };
    Ok(Json(rows.into_iter().map(PositionBody::from).collect()))
}

#[utoipa::path(
    put,
    path = "/api/positions",
    tag = "positions",
    request_body = Vec<SetPosition>,
    responses(
        (status = 200, description = "The listed positions were written. Other strategy books are left as they are", body = Vec<PositionBody>),
        (status = 422, description = "A strategy identifier, instrument, or price is not valid", body = ErrorBody),
        (status = 500, description = "The book could not be written", body = ErrorBody)
    )
)]
async fn set_positions(
    State(desk): State<Shared>,
    Json(input): Json<Vec<SetPosition>>,
) -> Result<Json<Vec<PositionBody>>, ApiError> {
    let positions = parse_set(input).map_err(DeskError::Invalid)?;
    let mut desk = lock(&desk)?;
    let stored = desk.set_positions(positions)?;
    Ok(Json(stored.into_iter().map(PositionBody::from).collect()))
}

fn parse_set(rows: Vec<SetPosition>) -> Result<Vec<Position>, String> {
    if rows.is_empty() {
        return Err("At least one position is required.".into());
    }
    let mut parsed = Vec::with_capacity(rows.len());
    let mut seen = std::collections::BTreeSet::new();
    for row in rows {
        let position = row.parse()?;
        let key = (position.strategy.clone(), position.instrument_id.clone());
        if !seen.insert(key) {
            return Err(format!(
                "{} is listed more than once for {}.",
                position.instrument_id, position.strategy
            ));
        }
        parsed.push(position);
    }
    Ok(parsed)
}

#[utoipa::path(
    get,
    path = "/api/orders",
    tag = "orders",
    responses(
        (status = 200, description = "Orders newest first, including ones the desk refused to forward", body = Vec<OrderBody>),
        (status = 500, description = "The book could not be read", body = ErrorBody)
    )
)]
async fn orders(State(desk): State<Shared>) -> Result<Json<Vec<OrderBody>>, ApiError> {
    let desk = lock(&desk)?;
    let rows = desk.orders()?.into_iter().map(OrderBody::from).collect();
    Ok(Json(rows))
}

#[utoipa::path(
    get,
    path = "/api/fills",
    tag = "fills",
    responses(
        (status = 200, description = "Fills newest first", body = Vec<FillBody>),
        (status = 500, description = "The book could not be read", body = ErrorBody)
    )
)]
async fn fills(State(desk): State<Shared>) -> Result<Json<Vec<FillBody>>, ApiError> {
    let desk = lock(&desk)?;
    let rows = desk.fills()?.into_iter().map(FillBody::from).collect();
    Ok(Json(rows))
}

#[utoipa::path(
    post,
    path = "/api/orders",
    tag = "orders",
    request_body = CreateOrder,
    responses(
        (status = 201, description = "The gateway filled the order and the strategy book was updated", body = SubmissionBody),
        (status = 422, description = "The order failed validation or a risk check", body = ErrorBody),
        (status = 502, description = "The gateway refused the order", body = ErrorBody),
        (status = 500, description = "The book could not be written", body = ErrorBody)
    )
)]
async fn create_order(
    State(desk): State<Shared>,
    Json(input): Json<CreateOrder>,
) -> Result<(StatusCode, Json<SubmissionBody>), ApiError> {
    let mut desk = lock(&desk)?;
    let submission = desk.submit(input)?;
    Ok((StatusCode::CREATED, Json(SubmissionBody::from(submission))))
}

fn lock(desk: &Shared) -> Result<std::sync::MutexGuard<'_, Desk>, ApiError> {
    desk.lock()
        .map_err(|_| ApiError::storage("The position book lock was poisoned."))
}

#[derive(Serialize, ToSchema)]
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

#[derive(Serialize, ToSchema)]
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

#[derive(Serialize, ToSchema)]
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

#[derive(Serialize, ToSchema)]
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

#[derive(Serialize, ToSchema)]
struct ErrorBody {
    error: String,
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
        let body = Json(ErrorBody {
            error: self.message,
        });
        (self.status, body).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::ApiDoc;
    use utoipa::OpenApi;

    #[test]
    fn openapi_lists_the_book_routes() {
        let spec = ApiDoc::openapi();
        let paths = &spec.paths.paths;
        let positions = paths.get("/api/positions").expect("positions path");
        assert!(positions.get.is_some());
        assert!(positions.put.is_some());
        assert!(paths.contains_key("/api/fills"));
        let orders = paths.get("/api/orders").expect("orders path");
        assert!(orders.get.is_some());
        assert!(orders.post.is_some());
    }
}
