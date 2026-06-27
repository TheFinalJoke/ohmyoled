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
    /// Playoff context from ESPN — a round/game headline (e.g. "East
    /// Semifinals Game 5") optionally followed by the series score ("Series
    /// tied 2-2"). `None` for regular-season games, so renderers show it only
    /// in the postseason.
    pub playoff_note: Option<String>,
}

/// How long after a final whistle we keep showing the score before treating the
/// slot as "no current game". Covers the gap between playoff series too.
const RECENT_FINAL_GRACE: chrono::Duration = chrono::Duration::days(2);

/// How far out a scheduled game can be and still count as the team's *current*
/// next game. In-season the next game is always days away (bye weeks ≤ 14d);
/// between seasons ESPN's `next_event` points months out, which this excludes.
const UPCOMING_HORIZON: chrono::Duration = chrono::Duration::days(21);

impl NextGame {
    /// Whether this game is worth showing *now* as the team's current game.
    ///
    /// ESPN's team `next_event` doesn't go empty in the off-season — it keeps
    /// returning the last final score (for weeks) or the first game of next
    /// season (months out). Both render as a misleading "current" scoreboard, so
    /// we treat a game as current only when it's actually live, imminent, or just
    /// finished.
    pub fn is_current(&self, now: DateTime<Local>) -> bool {
        match self.status {
            GameStatus::InProgress => true,
            GameStatus::Scheduled => {
                self.start <= now + UPCOMING_HORIZON && self.start >= now - RECENT_FINAL_GRACE
            }
            GameStatus::Final | GameStatus::Postponed => self.start >= now - RECENT_FINAL_GRACE,
        }
    }
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
    /// The game to actually display, or `None` when the only game ESPN gave us
    /// is stale (off-season). Renderers must branch on this rather than on
    /// `next_game` directly so a weeks-old final score doesn't read as current.
    pub fn current_game(&self, now: DateTime<Local>) -> Option<&NextGame> {
        self.next_game.as_ref().filter(|g| g.is_current(now))
    }

    /// `true` when there's no *current* game and no standings to scroll — the
    /// renderer falls back to a "{Sport} Offseason" placeholder. Evaluated
    /// against `now` so a stale `next_event` from ESPN reads as off-season.
    pub fn is_offseason_at(&self, now: DateTime<Local>) -> bool {
        self.current_game(now).is_none() && self.standings.is_empty()
    }

    /// Convenience wrapper over [`Self::is_offseason_at`] using the wall clock.
    pub fn is_offseason(&self) -> bool {
        self.is_offseason_at(Local::now())
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

    fn side(name: &str) -> TeamSide {
        TeamSide { name: name.into(), abbreviation: name.into(), logo_url: None, score: Some(0) }
    }

    fn game(start: DateTime<Local>, status: GameStatus) -> NextGame {
        NextGame {
            start,
            status,
            home: side("A"),
            away: side("B"),
            our_side: HomeOrAway::Home,
            playoff_note: None,
        }
    }

    fn data_with(game: Option<NextGame>, standings: bool) -> SportData {
        SportData {
            api: SportApiSource::Espn,
            sport: SportKind::Basketball,
            team_name: "A".into(),
            record: "1-0".into(),
            next_game: game,
            standings: if standings {
                vec![StandingsEntry { position: 1, team_name: "A".into() }]
            } else {
                vec![]
            },
        }
    }

    #[test]
    fn in_progress_is_always_current() {
        let now = Local::now();
        // Even a weirdly-dated live game shows — the league is clearly active.
        let g = game(now - chrono::Duration::days(40), GameStatus::InProgress);
        assert!(g.is_current(now));
    }

    #[test]
    fn imminent_scheduled_is_current_but_far_future_is_not() {
        let now = Local::now();
        assert!(game(now + chrono::Duration::days(3), GameStatus::Scheduled).is_current(now));
        // Next season's opener months out — off-season, not a current game.
        assert!(!game(now + chrono::Duration::days(90), GameStatus::Scheduled).is_current(now));
    }

    #[test]
    fn recent_final_is_current_but_stale_final_is_not() {
        let now = Local::now();
        // Just-finished game: keep the score up briefly.
        assert!(game(now - chrono::Duration::hours(6), GameStatus::Final).is_current(now));
        // A weeks-old final is what ESPN serves all off-season — must read stale.
        assert!(!game(now - chrono::Duration::days(20), GameStatus::Final).is_current(now));
    }

    #[test]
    fn stale_final_reads_as_offseason() {
        let now = Local::now();
        let stale = game(now - chrono::Duration::days(30), GameStatus::Final);
        let d = data_with(Some(stale), false);
        // The bug: next_game is Some, but it's stale → no current game → off-season.
        assert!(d.current_game(now).is_none());
        assert!(d.is_offseason_at(now));
    }

    #[test]
    fn playoff_series_active_keeps_showing_games() {
        let now = Local::now();
        // Mid-series: next game is a couple days out → current.
        let g = game(now + chrono::Duration::days(2), GameStatus::Scheduled);
        let d = data_with(Some(g), true);
        assert!(d.current_game(now).is_some());
        assert!(!d.is_offseason_at(now));
    }

    #[test]
    fn knocked_out_team_falls_to_offseason_after_grace() {
        let now = Local::now();
        // Team lost its elimination game; ESPN serves only that final, no next.
        // Within the grace window the final still shows…
        let just_lost = game(now - chrono::Duration::hours(12), GameStatus::Final);
        assert!(data_with(Some(just_lost), false).current_game(now).is_some());
        // …but a few days after elimination there is no current game.
        let eliminated = game(now - chrono::Duration::days(5), GameStatus::Final);
        let d = data_with(Some(eliminated), false);
        assert!(d.current_game(now).is_none());
        assert!(d.is_offseason_at(now));
    }
}
