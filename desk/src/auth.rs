use axum::extract::{Request, State};
use axum::http::{header, HeaderMap, Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Redirect, Response};
use axum::Json;
use serde_json::json;
use uuid::Uuid;

const COOKIE: &str = "desk_session";

#[derive(Clone)]
pub struct Gate {
    password: String,
    token: String,
}

impl Gate {
    pub fn new(password: String) -> Self {
        Self {
            password,
            token: Uuid::new_v4().simple().to_string(),
        }
    }

    pub fn accepts(&self, password: &str) -> bool {
        constant_time_eq(password.as_bytes(), self.password.as_bytes())
    }

    pub fn session_cookie(&self) -> String {
        format!(
            "{COOKIE}={}; HttpOnly; SameSite=Lax; Path=/; Max-Age=43200",
            self.token
        )
    }

    pub fn clear_cookie() -> &'static str {
        "desk_session=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0"
    }

    pub fn has_session(&self, headers: &HeaderMap) -> bool {
        cookie(headers)
            .is_some_and(|value| constant_time_eq(value.as_bytes(), self.token.as_bytes()))
    }
}

pub async fn guard(State(gate): State<Gate>, request: Request, next: Next) -> Response {
    let path = request.uri().path().to_owned();
    let method = request.method().clone();
    if is_open(&method, &path) || gate.has_session(request.headers()) {
        return next.run(request).await;
    }
    if path.starts_with("/api/") {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Sign in to the desk." })),
        )
            .into_response();
    }
    Redirect::to("/").into_response()
}

fn is_open(method: &Method, path: &str) -> bool {
    if *method == Method::POST && (path == "/api/login" || path == "/api/logout") {
        return true;
    }
    !path.starts_with("/api/") && !path.starts_with("/docs")
}

fn cookie(headers: &HeaderMap) -> Option<String> {
    let header = headers.get(header::COOKIE)?.to_str().ok()?;
    for part in header.split(';') {
        let part = part.trim();
        if let Some(value) = part.strip_prefix(&format!("{COOKIE}=")) {
            return Some(value.to_string());
        }
    }
    None
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    let mut diff = left.len() ^ right.len();
    for (left, right) in left.iter().zip(right.iter()) {
        diff |= (*left ^ *right) as usize;
    }
    diff == 0
}
