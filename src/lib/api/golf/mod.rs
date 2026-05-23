//! Golf collector — currently ESPN's free PGA scoreboard.

use crate::api::collector::Collector;
use crate::api::error::ApiError;
use async_trait::async_trait;
use std::time::Duration;

pub mod espn;

pub use espn::EspnGolfProvider;

use serde::{Deserialize, Serialize};

/// Tour to follow. ESPN's URL segment matches the lowercased name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GolfTour {
    Pga,
    Lpga,
    Champions,
    Korn,
    Liv,
}

impl GolfTour {
    pub fn espn_segment(self) -> &'static str {
        match self {
            Self::Pga => "pga",
            Self::Lpga => "lpga",
            Self::Champions => "champions-tour",
            Self::Korn => "web.com",
            Self::Liv => "liv",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Pga => "PGA",
            Self::Lpga => "LPGA",
            Self::Champions => "CHMP",
            Self::Korn => "KORN",
            Self::Liv => "LIV",
        }
    }
}

/// One leaderboard line.
#[derive(Debug, Clone)]
pub struct LeaderboardEntry {
    pub position: u32,
    pub player_short: String,
    /// Relative-to-par as ESPN returns it: e.g. `"-6"`, `"+2"`, `"E"`.
    pub score: String,
}

/// Normalized golf snapshot.
#[derive(Debug, Clone)]
pub struct GolfData {
    pub tour: GolfTour,
    pub event_name: String,
    /// `"In Progress"`, `"Final"`, `"Scheduled"`, etc. — copied from ESPN.
    pub status: String,
    pub leaderboard: Vec<LeaderboardEntry>,
}

impl GolfData {
    pub fn is_offseason(&self) -> bool {
        self.leaderboard.is_empty()
    }
}

pub enum GolfSource {
    Espn(EspnGolfProvider),
}

impl GolfSource {
    pub async fn poll(&self) -> Result<GolfData, ApiError> {
        match self {
            Self::Espn(p) => p.poll().await,
        }
    }
}

pub struct GolfCollector {
    source: GolfSource,
    refresh: Duration,
}

impl GolfCollector {
    pub fn new(source: GolfSource) -> Self {
        Self {
            source,
            refresh: Duration::from_secs(120),
        }
    }

    pub fn from_espn(tour: GolfTour) -> Self {
        Self::new(GolfSource::Espn(EspnGolfProvider::new(tour)))
    }
}

#[async_trait]
impl Collector for GolfCollector {
    type Output = GolfData;

    fn id(&self) -> &'static str {
        "golf"
    }

    fn refresh_interval(&self) -> Duration {
        self.refresh
    }

    async fn poll(&self) -> Result<GolfData, ApiError> {
        self.source.poll().await
    }
}
