//! Formula 1 collector — jolpica's free Ergast-compatible API.
//!
//! Ergast (`ergast.com/api/f1`) was deprecated in 2024; `api.jolpi.ca/ergast/f1`
//! is a drop-in replacement maintained by the jolpica project. Same URL shape
//! and JSON schema, no auth, generous rate limit.

use crate::api::collector::Collector;
use crate::api::error::ApiError;
use async_trait::async_trait;
use chrono::{DateTime, Local};
use std::time::Duration;

pub mod jolpica;

pub use jolpica::JolpicaProvider;

#[derive(Debug, Clone)]
pub struct NextRace {
    pub round: u32,
    pub name: String,
    pub circuit: String,
    pub start: DateTime<Local>,
}

#[derive(Debug, Clone)]
pub struct DriverStanding {
    pub position: u32,
    pub code: String,
    pub family_name: String,
    pub points: f32,
}

#[derive(Debug, Clone)]
pub struct F1Data {
    pub season: String,
    pub next_race: Option<NextRace>,
    pub standings: Vec<DriverStanding>,
}

impl F1Data {
    /// `true` when there's no upcoming race scheduled. Standings may still be
    /// populated (last season's table is what jolpica returns between
    /// seasons), so the renderer uses that to show a champion banner.
    pub fn is_offseason(&self) -> bool {
        self.next_race.is_none()
    }
}

pub enum F1Source {
    Jolpica(JolpicaProvider),
}

impl F1Source {
    pub async fn poll(&self) -> Result<F1Data, ApiError> {
        match self {
            Self::Jolpica(p) => p.poll().await,
        }
    }
}

pub struct F1Collector {
    source: F1Source,
    refresh: Duration,
}

impl F1Collector {
    pub fn new(source: F1Source) -> Self {
        Self {
            source,
            refresh: Duration::from_secs(300),
        }
    }

    pub fn from_jolpica() -> Self {
        Self::new(F1Source::Jolpica(JolpicaProvider::new()))
    }
}

#[async_trait]
impl Collector for F1Collector {
    type Output = F1Data;

    fn id(&self) -> &'static str {
        "f1"
    }

    fn refresh_interval(&self) -> Duration {
        self.refresh
    }

    async fn poll(&self) -> Result<F1Data, ApiError> {
        self.source.poll().await
    }
}
