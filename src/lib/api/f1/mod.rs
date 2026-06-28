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

/// Furthest out a `next_race` can be and still count as the *current* upcoming
/// race. The longest in-season gap (the summer break) is ~4 weeks; between
/// seasons jolpica's `/next` points at the new season's opener months out, which
/// this excludes so the winter break reads as off-season (champion banner) and
/// the countdown doesn't tick toward a race in March.
const NEXT_RACE_HORIZON: chrono::Duration = chrono::Duration::days(45);

impl F1Data {
    /// The race to actually count down to, or `None` when the only race jolpica
    /// gave us is next season's (months out → off-season). Renderers branch on
    /// this rather than `next_race` directly.
    pub fn current_race(&self, now: DateTime<Local>) -> Option<&NextRace> {
        self.next_race.as_ref().filter(|r| r.start <= now + NEXT_RACE_HORIZON)
    }

    /// `true` when there's no *current* upcoming race. Standings may still be
    /// populated (last season's table is what jolpica returns between seasons),
    /// so the renderer uses that to show a champion banner. Evaluated against
    /// `now` so a far-future winter-break race reads as off-season.
    pub fn is_offseason_at(&self, now: DateTime<Local>) -> bool {
        self.current_race(now).is_none()
    }

    /// Convenience wrapper over [`Self::is_offseason_at`] using the wall clock.
    pub fn is_offseason(&self) -> bool {
        self.is_offseason_at(Local::now())
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

#[cfg(test)]
mod tests {
    use super::*;

    fn race_at(start: DateTime<Local>) -> NextRace {
        NextRace { round: 1, name: "GP".into(), circuit: "Track".into(), start }
    }

    #[test]
    fn imminent_race_is_current() {
        let now = Local::now();
        let d = F1Data {
            season: "2026".into(),
            next_race: Some(race_at(now + chrono::Duration::days(7))),
            standings: vec![],
        };
        assert!(d.current_race(now).is_some());
        assert!(!d.is_offseason_at(now));
    }

    #[test]
    fn summer_break_gap_still_counts_as_current() {
        // The longest in-season gap is ~4 weeks — must NOT read as off-season.
        let now = Local::now();
        let d = F1Data {
            season: "2026".into(),
            next_race: Some(race_at(now + chrono::Duration::days(28))),
            standings: vec![],
        };
        assert!(!d.is_offseason_at(now));
    }

    #[test]
    fn next_season_opener_reads_as_offseason() {
        // Winter break: jolpica points at March; months out → off-season.
        let now = Local::now();
        let d = F1Data {
            season: "2026".into(),
            next_race: Some(race_at(now + chrono::Duration::days(100))),
            standings: vec![],
        };
        assert!(d.current_race(now).is_none());
        assert!(d.is_offseason_at(now));
    }
}
