mod api;
mod auth;
#[cfg(test)]
mod broker;
mod model;
#[cfg(test)]
mod position;
#[cfg(test)]
mod risk;
mod store;

use std::path::Path;
use std::sync::{Arc, Mutex};

use axum::Router;

use crate::store::Desk;

pub fn router(path: impl AsRef<Path>, password: String) -> Result<Router, store::DeskError> {
    let desk = Desk::open(path)?;
    Ok(api::router(Arc::new(Mutex::new(desk)), password))
}
