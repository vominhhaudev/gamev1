use common_net::telemetry;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod api;
pub mod collections;
pub mod jobs;
pub mod persistence;

fn main() {
    telemetry::init("services");
    tracing::info!("services runner started");
}
