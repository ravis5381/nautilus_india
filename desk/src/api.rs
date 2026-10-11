use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use tower_http::services::{ServeDir, ServeFile};
use utoipa::{IntoParams, OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;

use crate::model::{money, normalize_strategy_name, Position, SetPosition, Side};
use crate::store::{Desk, DeskError};

pub type Shared = Arc<Mutex<Desk>>;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "India desk",
        version = "0.1.0",
        description = "Position book. Each strategy keeps its own quantity for an instrument."
    ),
    paths(positions, set_positions),
    components(schemas(PositionBody, SetPosition, ErrorBody, Side)),
    tags((name = "positions", description = "Strategy books"))
)]
struct ApiDoc;

pub fn router(desk: Shared) -> Router {
    let dist = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("web/dist");
    let index = dist.join("index.html");
    let api = Router::new()
        .route("/positions", get(positions).put(set_positions))
        .fallback(api_missing)
        .with_state(desk);

    Router::new()
        .nest("/api", api)
        .merge(SwaggerUi::new("/docs").url("/api/openapi.json", ApiDoc::openapi()))
        .route("/docs/index.css", get(docs_css))
        .fallback_service(ServeDir::new(dist).not_found_service(ServeFile::new(index)))
}

async fn docs_css() -> impl IntoResponse {
    ([("content-type", "text/css; charset=utf-8")], DOCS_CSS)
}

const DOCS_CSS: &str = r#"
html {
    box-sizing: border-box;
    overflow: -moz-scrollbars-vertical;
    overflow-y: scroll;
}

*,
*:before,
*:after {
    box-sizing: inherit;
}

body {
    margin: 0;
    background: #fafafa;
}

.swagger-ui .operation-servers {
    display: none;
}
"#;

async fn api_missing() -> StatusCode {
    StatusCode::NOT_FOUND
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
        assert!(!paths.contains_key("/api/orders"));
        assert!(!paths.contains_key("/api/fills"));
    }
}
