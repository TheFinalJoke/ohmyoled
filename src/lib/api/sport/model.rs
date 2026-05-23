//! Normalized sport snapshot — what the collector hands the renderer.
//!
//! Direct shape-port of `src/python/ohmyoled/lib/sports/sportbase.py`,
//! minus the unused parts (`SportApiResult`, multi-API normalization layer).

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

/// Which API produced this data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SportApiSource {
    /// ESPN's free public/unofficial API at `site.api.espn.com`.
    Espn,
}

/// Sport / league this snapshot describes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SportKind {
    Baseball,
    Basketball,
    Football,
    Hockey,
}

impl SportKind {
    /// ESPN URL segment for this sport (matches its `/sports/{sport}/...` convention).
    pub fn espn_sport(self) -> &'static str {
        match self {
            Self::Baseball => "baseball",
            Self::Basketball => "basketball",
            Self::Football => "football",
            Self::Hockey => "hockey",
        }
    }

    /// ESPN league code under that sport.
    pub fn espn_league(self) -> &'static str {
        match self {
            Self::Baseball => "mlb",
            Self::Basketball => "nba",
            Self::Football => "nfl",
            Self::Hockey => "nhl",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Baseball => "MLB",
            Self::Basketball => "NBA",
            Self::Football => "NFL",
            Self::Hockey => "NHL",
        }
    }
}

/// Where the configured team is in this game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HomeOrAway {
    Home,
    Away,
}

/// State of a game — mirrors the Python `GameStatus` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameStatus {
    Scheduled,
    InProgress,
    Final,
    Postponed,
}

/// One team within a game (ours or the opponent).
#[derive(Debug, Clone)]
pub struct TeamSide {
    pub name: String,
    pub abbreviation: String,
    pub logo_url: Option<String>,
    pub score: Option<i32>,
}

/// The game we want to display — next upcoming, in progress, or just finished.
#[derive(Debug, Clone)]
pub struct NextGame {
    pub start: DateTime<Local>,
    pub status: GameStatus,
    pub home: TeamSide,
    pub away: TeamSide,
    /// Which side is *our* configured team.
    pub our_side: HomeOrAway,
}

/// One row in the league standings table.
#[derive(Debug, Clone)]
pub struct StandingsEntry {
    pub position: u32,
    pub team_name: String,
}

/// Top-level normalized sport payload.
#[derive(Debug, Clone)]
pub struct SportData {
    pub api: SportApiSource,
    pub sport: SportKind,
    pub team_name: String,
    /// "56-26" style overall record for the configured team.
    pub record: String,
    pub next_game: Option<NextGame>,
    pub standings: Vec<StandingsEntry>,
}

impl SportData {
    /// `true` when there's no upcoming game and no standings to scroll — the
    /// renderer falls back to a "{Sport} Offseason" placeholder.
    pub fn is_offseason(&self) -> bool {
        self.next_game.is_none() && self.standings.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sport_kind_espn_paths() {
        assert_eq!(SportKind::Basketball.espn_sport(), "basketball");
        assert_eq!(SportKind::Basketball.espn_league(), "nba");
        assert_eq!(SportKind::Hockey.espn_league(), "nhl");
        assert_eq!(SportKind::Football.espn_league(), "nfl");
        assert_eq!(SportKind::Baseball.espn_league(), "mlb");
    }

    #[test]
    fn offseason_when_no_data() {
        let d = SportData {
            api: SportApiSource::Espn,
            sport: SportKind::Baseball,
            team_name: "Test".into(),
            record: "0-0".into(),
            next_game: None,
            standings: vec![],
        };
        assert!(d.is_offseason());
    }
}
