//! ESPN unofficial API provider.
//!
//! ESPN serves a public, unauthenticated JSON API at `site.api.espn.com`. It
//! isn't documented as a stable contract, but it's been the de-facto
//! free-tier sports source for years and gives us everything the panel
//! renderer needs: next game with scores, team record, and league standings.
//!
//! Two endpoints used here:
//! - `/apis/site/v2/sports/{sport}/{league}/teams/{id}` — current team page,
//!   includes `nextEvent[]` with `competitions[].competitors[]` (home/away,
//!   scores, status), plus `record.items[0].summary`.
//! - `/apis/v2/sports/{sport}/{league}/standings` — conferences → entries.
//!
//! ESPN uses its own numeric team IDs which don't match the `apisportsid` or
//! `sportsdbid` we already track. On first poll we hit `/teams` (no key), find
//! the configured team by name/abbreviation, and cache the resolved ID.

use super::model::{
    GameStatus, HomeOrAway, NextGame, SportApiSource, SportData, SportKind, StandingsEntry,
    TeamSide,
};
use crate::api::error::ApiError;
use crate::api::http::get_json;
use chrono::{DateTime, Local, NaiveDateTime, TimeZone, Utc};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::OnceCell;

#[derive(Debug, Clone)]
pub struct EspnConfig {
    pub sport: SportKind,
    /// Full team name as it appears in the config (e.g. `"Boston Celtics"`).
    pub team_name: String,
    /// Three-letter abbreviation (e.g. `"BOS"`). Used as a tiebreaker if the
    /// full name lookup fails.
    pub team_abbreviation: String,
}

pub struct EspnProvider {
    cfg: EspnConfig,
    team_id: Arc<OnceCell<String>>,
}

impl EspnProvider {
    pub fn new(cfg: EspnConfig) -> Self {
        Self {
            cfg,
            team_id: Arc::new(OnceCell::new()),
        }
    }

    /// Resolve `team_name` → ESPN's numeric team ID, caching the result.
    async fn resolve_team_id(&self) -> Result<String, ApiError> {
        if let Some(id) = self.team_id.get() {
            return Ok(id.clone());
        }
        let url = format!(
            "https://site.api.espn.com/apis/site/v2/sports/{}/{}/teams",
            self.cfg.sport.espn_sport(),
            self.cfg.sport.espn_league()
        );
        let resp: TeamsListResponse = get_json(&url, &[]).await?;

        let needle_name = self.cfg.team_name.to_lowercase();
        let needle_abbr = self.cfg.team_abbreviation.to_uppercase();
        let found = resp
            .sports
            .into_iter()
            .flat_map(|s| s.leagues)
            .flat_map(|l| l.teams)
            .map(|entry| entry.team)
            .find(|t| {
                t.display_name.to_lowercase() == needle_name
                    || t.abbreviation.to_uppercase() == needle_abbr
            })
            .ok_or_else(|| ApiError::Provider {
                provider: "espn",
                msg: format!(
                    "team '{}' (abbr '{}') not found in {}/{}",
                    self.cfg.team_name,
                    self.cfg.team_abbreviation,
                    self.cfg.sport.espn_sport(),
                    self.cfg.sport.espn_league()
                ),
            })?;

        let _ = self.team_id.set(found.id.clone());
        Ok(found.id)
    }

    pub async fn poll(&self) -> Result<SportData, ApiError> {
        let team_id = self.resolve_team_id().await?;

        let team_url = format!(
            "https://site.api.espn.com/apis/site/v2/sports/{}/{}/teams/{}",
            self.cfg.sport.espn_sport(),
            self.cfg.sport.espn_league(),
            team_id
        );
        let standings_url = format!(
            "https://site.api.espn.com/apis/v2/sports/{}/{}/standings",
            self.cfg.sport.espn_sport(),
            self.cfg.sport.espn_league()
        );

        let (team_res, standings_res) = tokio::join!(
            get_json::<TeamDetailResponse>(&team_url, &[]),
            get_json::<StandingsResponse>(&standings_url, &[]),
        );

        let team_detail = team_res?;
        let standings = standings_res.unwrap_or_else(|e| {
            log::warn!("espn: standings fetch failed, continuing without: {e}");
            StandingsResponse { children: vec![] }
        });

        normalize(self.cfg.sport, &self.cfg.team_name, team_detail, standings)
    }
}

// ---------------------------------------------------------------------------
// Response shapes — only the fields we actually consume.
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct TeamsListResponse {
    sports: Vec<SportEntry>,
}

#[derive(Debug, Deserialize)]
struct SportEntry {
    leagues: Vec<LeagueEntry>,
}

#[derive(Debug, Deserialize)]
struct LeagueEntry {
    teams: Vec<TeamWrapper>,
}

#[derive(Debug, Deserialize)]
struct TeamWrapper {
    team: TeamSummary,
}

#[derive(Debug, Deserialize)]
struct TeamSummary {
    id: String,
    #[serde(rename = "displayName")]
    display_name: String,
    #[serde(default)]
    abbreviation: String,
}

#[derive(Debug, Deserialize)]
struct TeamDetailResponse {
    team: TeamDetail,
}

#[derive(Debug, Deserialize)]
struct TeamDetail {
    #[serde(rename = "displayName")]
    display_name: String,
    #[serde(default)]
    record: RecordWrapper,
    #[serde(rename = "nextEvent", default)]
    next_event: Vec<EventEntry>,
}

#[derive(Debug, Default, Deserialize)]
struct RecordWrapper {
    #[serde(default)]
    items: Vec<RecordItem>,
}

#[derive(Debug, Deserialize)]
struct RecordItem {
    #[serde(default)]
    summary: String,
}

#[derive(Debug, Deserialize)]
struct EventEntry {
    date: String,
    #[serde(default)]
    competitions: Vec<Competition>,
}

#[derive(Debug, Deserialize)]
struct Competition {
    #[serde(default)]
    status: Option<StatusWrapper>,
    #[serde(default)]
    competitors: Vec<Competitor>,
    /// Playoff round/game headlines (e.g. "East 1st Round - Game 5"). Present
    /// only in the postseason; empty otherwise.
    #[serde(default)]
    notes: Vec<CompetitionNote>,
    /// Playoff series state, when this game is part of a series.
    #[serde(default)]
    series: Option<SeriesInfo>,
}

#[derive(Debug, Deserialize)]
struct CompetitionNote {
    #[serde(default)]
    headline: String,
}

#[derive(Debug, Deserialize)]
struct SeriesInfo {
    /// e.g. "Series tied 2-2" / "BOS leads 3-2". ESPN names this `summary`.
    #[serde(default)]
    summary: String,
}

#[derive(Debug, Deserialize)]
struct StatusWrapper {
    #[serde(rename = "type")]
    inner: StatusInner,
}

#[derive(Debug, Deserialize)]
struct StatusInner {
    #[serde(default)]
    state: String,
    #[serde(default)]
    completed: bool,
    #[serde(default)]
    description: String,
}

#[derive(Debug, Deserialize)]
struct Competitor {
    #[serde(rename = "homeAway", default)]
    home_away: String,
    team: CompetitorTeam,
    #[serde(default)]
    score: ScoreValue,
}

#[derive(Debug, Deserialize)]
struct CompetitorTeam {
    #[serde(rename = "displayName")]
    display_name: String,
    #[serde(default)]
    abbreviation: String,
    #[serde(default)]
    logos: Vec<LogoEntry>,
}

#[derive(Debug, Deserialize)]
struct LogoEntry {
    href: String,
}

/// ESPN sometimes serializes a competitor's score as `{"value": 100.0, ...}`
/// and sometimes as a plain string `"100"`. Accept both.
#[derive(Debug, Default)]
struct ScoreValue(Option<i32>);

impl<'de> Deserialize<'de> for ScoreValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;
        let raw = serde_json::Value::deserialize(deserializer)?;
        let v = match raw {
            serde_json::Value::Null => None,
            serde_json::Value::Number(n) => n.as_f64().map(|f| f.round() as i32),
            serde_json::Value::String(s) => s.parse::<i32>().ok(),
            serde_json::Value::Object(map) => map
                .get("value")
                .and_then(|v| v.as_f64())
                .map(|f| f.round() as i32),
            other => return Err(D::Error::custom(format!("unexpected score shape: {other}"))),
        };
        Ok(Self(v))
    }
}

#[derive(Debug, Deserialize)]
struct StandingsResponse {
    #[serde(default)]
    children: Vec<StandingsGroup>,
}

#[derive(Debug, Deserialize)]
struct StandingsGroup {
    #[serde(default)]
    standings: StandingsBlock,
}

#[derive(Debug, Default, Deserialize)]
struct StandingsBlock {
    #[serde(default)]
    entries: Vec<StandingsEntryRaw>,
}

#[derive(Debug, Deserialize)]
struct StandingsEntryRaw {
    team: StandingsTeam,
}

#[derive(Debug, Deserialize)]
struct StandingsTeam {
    #[serde(rename = "displayName")]
    display_name: String,
}

// ---------------------------------------------------------------------------
// Normalization.
// ---------------------------------------------------------------------------

fn normalize(
    sport: SportKind,
    configured_team: &str,
    detail: TeamDetailResponse,
    standings: StandingsResponse,
) -> Result<SportData, ApiError> {
    let our_team_lower = configured_team.to_lowercase();
    let team = detail.team;
    let record = team
        .record
        .items
        .into_iter()
        .next()
        .map(|i| i.summary)
        .unwrap_or_default();

    let next_game = team.next_event.into_iter().next().and_then(|e| {
        let start = parse_espn_date(&e.date)?;
        let comp = e.competitions.into_iter().next()?;
        let status = comp
            .status
            .as_ref()
            .map(|s| match s.inner.state.as_str() {
                "in" => GameStatus::InProgress,
                "post" if s.inner.completed => GameStatus::Final,
                "post" => GameStatus::Postponed,
                _ => GameStatus::Scheduled,
            })
            .unwrap_or(GameStatus::Scheduled);
        let _status_desc = comp.status.map(|s| s.inner.description).unwrap_or_default();

        let mut home: Option<TeamSide> = None;
        let mut away: Option<TeamSide> = None;
        for c in comp.competitors {
            let side = TeamSide {
                name: c.team.display_name.clone(),
                abbreviation: c.team.abbreviation.clone(),
                logo_url: c.team.logos.into_iter().next().map(|l| l.href),
                score: c.score.0,
            };
            match c.home_away.as_str() {
                "home" => home = Some(side),
                "away" => away = Some(side),
                _ => {}
            }
        }
        let home = home?;
        let away = away?;
        let our_side = if home.name.to_lowercase() == our_team_lower {
            HomeOrAway::Home
        } else {
            HomeOrAway::Away
        };
        // Postseason context: the round/game headline, plus the series score
        // when present. Both are empty in the regular season → `None`.
        let round = comp.notes.into_iter().map(|n| n.headline).find(|h| !h.is_empty());
        let series = comp.series.map(|s| s.summary).filter(|s| !s.is_empty());
        let playoff_note = match (round, series) {
            (Some(r), Some(s)) => Some(format!("{r} · {s}")),
            (Some(r), None) => Some(r),
            (None, Some(s)) => Some(s),
            (None, None) => None,
        };
        Some(NextGame { start, status, home, away, our_side, playoff_note })
    });

    let standings_entries = standings
        .children
        .into_iter()
        .flat_map(|c| c.standings.entries)
        .enumerate()
        .map(|(idx, e)| StandingsEntry {
            position: (idx as u32) + 1,
            team_name: e.team.display_name,
        })
        .collect();

    Ok(SportData {
        api: SportApiSource::Espn,
        sport,
        team_name: team.display_name,
        record,
        next_game,
        standings: standings_entries,
    })
}

/// ESPN's `date` field comes in flavours like `2026-05-02T23:30Z` (no seconds)
/// or `2026-05-02T23:30:00Z`. RFC 3339 demands seconds, so fall through several
/// formats and finally bail to `None`.
fn parse_espn_date(s: &str) -> Option<DateTime<Local>> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&Local));
    }
    for fmt in &["%Y-%m-%dT%H:%MZ", "%Y-%m-%dT%H:%M:%SZ", "%Y-%m-%dT%H:%M"] {
        if let Ok(naive) = NaiveDateTime::parse_from_str(s, fmt) {
            // ESPN's `Z` form is UTC; assume that for the no-tz suffix too.
            return Some(Utc.from_utc_datetime(&naive).with_timezone(&Local));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Trimmed real-shape response from `/teams/2` (Boston Celtics).
    const TEAM_FIXTURE: &str = r#"
    {
      "team": {
        "id": "2",
        "displayName": "Boston Celtics",
        "abbreviation": "BOS",
        "record": { "items": [{ "summary": "56-26" }] },
        "nextEvent": [{
          "date": "2026-05-02T23:30Z",
          "competitions": [{
            "status": { "type": { "state": "post", "completed": true, "description": "Final" } },
            "competitors": [
              {
                "homeAway": "home",
                "team": { "displayName": "Boston Celtics", "abbreviation": "BOS", "logos": [{"href": "https://x/celtics.png"}] },
                "score": {"value": 100.0, "displayValue": "100"}
              },
              {
                "homeAway": "away",
                "team": { "displayName": "Philadelphia 76ers", "abbreviation": "PHI", "logos": [{"href": "https://x/sixers.png"}] },
                "score": {"value": 109.0, "displayValue": "109"}
              }
            ]
          }]
        }]
      }
    }
    "#;

    const STANDINGS_FIXTURE: &str = r#"
    {
      "children": [{
        "standings": {
          "entries": [
            {"team": {"displayName": "Cleveland Cavaliers"}},
            {"team": {"displayName": "Boston Celtics"}},
            {"team": {"displayName": "New York Knicks"}}
          ]
        }
      }]
    }
    "#;

    #[test]
    fn parses_team_fixture() {
        let team: TeamDetailResponse = serde_json::from_str(TEAM_FIXTURE).unwrap();
        let standings: StandingsResponse = serde_json::from_str(STANDINGS_FIXTURE).unwrap();
        let d = normalize(SportKind::Basketball, "Boston Celtics", team, standings).unwrap();
        assert_eq!(d.team_name, "Boston Celtics");
        assert_eq!(d.record, "56-26");
        let ng = d.next_game.expect("next_game");
        assert_eq!(ng.status, GameStatus::Final);
        assert_eq!(ng.our_side, HomeOrAway::Home);
        assert_eq!(ng.home.score, Some(100));
        assert_eq!(ng.away.name, "Philadelphia 76ers");
        assert_eq!(ng.away.score, Some(109));
        assert_eq!(d.standings.len(), 3);
        assert_eq!(d.standings[1].position, 2);
        assert_eq!(d.standings[1].team_name, "Boston Celtics");
    }

    #[test]
    fn score_value_accepts_string_form() {
        // ESPN's score field sometimes arrives as a plain string.
        let json = r#"{"homeAway":"home","team":{"displayName":"X","abbreviation":"X"},"score":"42"}"#;
        let c: Competitor = serde_json::from_str(json).unwrap();
        assert_eq!(c.score.0, Some(42));
    }

    #[test]
    fn score_value_accepts_null() {
        let json = r#"{"homeAway":"home","team":{"displayName":"X","abbreviation":"X"},"score":null}"#;
        let c: Competitor = serde_json::from_str(json).unwrap();
        assert_eq!(c.score.0, None);
    }

    #[test]
    fn parses_playoff_round_and_series_note() {
        // Postseason games carry a `notes` headline and a `series` block.
        let team_json = r#"
        {"team":{"displayName":"Boston Celtics","abbreviation":"BOS","record":{"items":[{"summary":"4-2"}]},
          "nextEvent":[{
            "date":"2026-05-12T23:30Z",
            "competitions":[{
              "status":{"type":{"state":"pre","completed":false,"description":"Scheduled"}},
              "notes":[{"headline":"East Semifinals - Game 5"}],
              "series":{"summary":"Series tied 2-2"},
              "competitors":[
                {"homeAway":"home","team":{"displayName":"Boston Celtics","abbreviation":"BOS"},"score":null},
                {"homeAway":"away","team":{"displayName":"New York Knicks","abbreviation":"NYK"},"score":null}
              ]
            }]
          }]
        }}"#;
        let team: TeamDetailResponse = serde_json::from_str(team_json).unwrap();
        let standings: StandingsResponse = serde_json::from_str(r#"{"children":[]}"#).unwrap();
        let d = normalize(SportKind::Basketball, "Boston Celtics", team, standings).unwrap();
        let ng = d.next_game.expect("next_game");
        assert_eq!(
            ng.playoff_note.as_deref(),
            Some("East Semifinals - Game 5 · Series tied 2-2")
        );
    }

    #[test]
    fn regular_season_game_has_no_playoff_note() {
        // No `notes`/`series` → regular season → note stays None.
        let team: TeamDetailResponse = serde_json::from_str(TEAM_FIXTURE).unwrap();
        let standings: StandingsResponse = serde_json::from_str(STANDINGS_FIXTURE).unwrap();
        let d = normalize(SportKind::Basketball, "Boston Celtics", team, standings).unwrap();
        assert!(d.next_game.unwrap().playoff_note.is_none());
    }

    #[test]
    fn empty_next_event_is_offseason_marker() {
        let team_json = r#"{"team":{"displayName":"X","abbreviation":"X","record":{"items":[]},"nextEvent":[]}}"#;
        let stand_json = r#"{"children":[]}"#;
        let team: TeamDetailResponse = serde_json::from_str(team_json).unwrap();
        let standings: StandingsResponse = serde_json::from_str(stand_json).unwrap();
        let d = normalize(SportKind::Hockey, "X", team, standings).unwrap();
        assert!(d.next_game.is_none());
        assert!(d.is_offseason());
    }
}
