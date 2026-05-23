//! Golf provider backed by ESPN's free `/sports/golf/{tour}/scoreboard` endpoint.

use super::{GolfData, GolfTour, LeaderboardEntry};
use crate::api::error::ApiError;
use crate::api::http::get_json;
use serde::Deserialize;

pub struct EspnGolfProvider {
    tour: GolfTour,
}

impl EspnGolfProvider {
    pub fn new(tour: GolfTour) -> Self {
        Self { tour }
    }

    pub async fn poll(&self) -> Result<GolfData, ApiError> {
        let url = format!(
            "https://site.api.espn.com/apis/site/v2/sports/golf/{}/scoreboard",
            self.tour.espn_segment()
        );
        let resp: ScoreboardResponse = get_json(&url, &[]).await?;
        Ok(normalize(self.tour, resp))
    }
}

// ---------------------------------------------------------------------------
// Response shapes — only what we read.
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct ScoreboardResponse {
    #[serde(default)]
    events: Vec<EventEntry>,
}

#[derive(Debug, Deserialize)]
struct EventEntry {
    #[serde(default)]
    name: String,
    #[serde(default)]
    status: Option<StatusWrapper>,
    #[serde(default)]
    competitions: Vec<Competition>,
}

#[derive(Debug, Deserialize)]
struct StatusWrapper {
    #[serde(rename = "type")]
    inner: StatusInner,
}

#[derive(Debug, Deserialize)]
struct StatusInner {
    #[serde(default)]
    description: String,
}

#[derive(Debug, Deserialize)]
struct Competition {
    #[serde(default)]
    competitors: Vec<Competitor>,
}

#[derive(Debug, Deserialize)]
struct Competitor {
    #[serde(default)]
    order: u32,
    #[serde(default)]
    score: String,
    athlete: Athlete,
}

#[derive(Debug, Deserialize)]
struct Athlete {
    #[serde(rename = "shortName", default)]
    short_name: String,
    #[serde(rename = "displayName", default)]
    display_name: String,
}

fn normalize(tour: GolfTour, resp: ScoreboardResponse) -> GolfData {
    let event = resp.events.into_iter().next();
    let (event_name, status, leaderboard) = match event {
        Some(e) => {
            let status = e
                .status
                .map(|s| s.inner.description)
                .unwrap_or_else(|| "Unknown".to_string());
            let comp = e.competitions.into_iter().next();
            let leaderboard = comp
                .map(|c| {
                    let mut entries: Vec<LeaderboardEntry> = c
                        .competitors
                        .into_iter()
                        .map(|c| LeaderboardEntry {
                            position: c.order,
                            player_short: if !c.athlete.short_name.is_empty() {
                                c.athlete.short_name
                            } else {
                                c.athlete.display_name
                            },
                            score: c.score,
                        })
                        .collect();
                    entries.sort_by_key(|e| e.position);
                    entries
                })
                .unwrap_or_default();
            (e.name, status, leaderboard)
        }
        None => (String::new(), "No Event".to_string(), vec![]),
    };

    GolfData { tour, event_name, status, leaderboard }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = r#"
    {
      "events": [{
        "name": "PGA Championship",
        "status": { "type": { "description": "In Progress" } },
        "competitions": [{
          "competitors": [
            {"order": 2, "score": "-4", "athlete": {"shortName": "M. Schmid", "displayName": "Matti Schmid"}},
            {"order": 1, "score": "-6", "athlete": {"shortName": "A. Smalley", "displayName": "Alex Smalley"}},
            {"order": 3, "score": "-4", "athlete": {"shortName": "N. Taylor", "displayName": "Nick Taylor"}}
          ]
        }]
      }]
    }
    "#;

    #[test]
    fn parses_and_sorts_by_position() {
        let resp: ScoreboardResponse = serde_json::from_str(FIXTURE).unwrap();
        let d = normalize(GolfTour::Pga, resp);
        assert_eq!(d.event_name, "PGA Championship");
        assert_eq!(d.status, "In Progress");
        assert_eq!(d.leaderboard.len(), 3);
        assert_eq!(d.leaderboard[0].position, 1);
        assert_eq!(d.leaderboard[0].player_short, "A. Smalley");
        assert_eq!(d.leaderboard[0].score, "-6");
        assert_eq!(d.leaderboard[2].player_short, "N. Taylor");
    }

    #[test]
    fn empty_scoreboard_is_offseason() {
        let resp: ScoreboardResponse = serde_json::from_str(r#"{"events": []}"#).unwrap();
        let d = normalize(GolfTour::Pga, resp);
        assert!(d.is_offseason());
    }

    #[test]
    fn tour_paths() {
        assert_eq!(GolfTour::Pga.espn_segment(), "pga");
        assert_eq!(GolfTour::Lpga.espn_segment(), "lpga");
        assert_eq!(GolfTour::Liv.display_name(), "LIV");
    }
}
