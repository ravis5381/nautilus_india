mod api;
mod broker;
mod model;
mod position;
mod risk;
mod store;

use std::path::Path;
use std::sync::{Arc, Mutex};

use axum::Router;

use crate::store::Desk;

pub fn router(path: impl AsRef<Path>) -> Result<Router, store::DeskError> {
    let desk = Desk::open(path)?;
    Ok(api::router(Arc::new(Mutex::new(desk))))
}
