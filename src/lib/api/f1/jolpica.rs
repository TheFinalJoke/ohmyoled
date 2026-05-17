//! Jolpica F1 provider (Ergast-compatible).
//!
//! Two endpoints fetched in parallel:
//! - `current/next.json` → next race name, circuit, date+time
//! - `current/driverStandings.json` → 22-deep driver standings table

use super::{DriverStanding, F1Data, NextRace};
use crate::api::error::ApiError;
use crate::api::http::get_json;
use chrono::{Local, NaiveDateTime, TimeZone, Utc};
use serde::Deserialize;

const NEXT_URL: &str = "https://api.jolpi.ca/ergast/f1/current/next.json";
const STANDINGS_URL: &str = "https://api.jolpi.ca/ergast/f1/current/driverStandings.json";

pub struct JolpicaProvider;

impl JolpicaProvider {
    pub fn new() -> Self {
        Self
    }

    pub async fn poll(&self) -> Result<F1Data, ApiError> {
        let (next_res, stand_res) = tokio::join!(
            get_json::<RaceTableResponse>(NEXT_URL, &[]),
            get_json::<StandingsResponse>(STANDINGS_URL, &[]),
        );
        let next = next_res?;
        let stand = stand_res?;
        Ok(normalize(next, stand))
    }
}

impl Default for JolpicaProvider {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Response shapes.
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct RaceTableResponse {
    #[serde(rename = "MRData")]
    mrdata: NextMRData,
}

#[derive(Debug, Deserialize)]
struct NextMRData {
    #[serde(rename = "RaceTable")]
    race_table: RaceTable,
}

#[derive(Debug, Deserialize)]
struct RaceTable {
    #[serde(rename = "season", default)]
    season: String,
    #[serde(rename = "Races", default)]
    races: Vec<RaceRaw>,
}

#[derive(Debug, Deserialize)]
struct RaceRaw {
    #[serde(default)]
    round: String,
    #[serde(rename = "raceName", default)]
    race_name: String,
    #[serde(rename = "Circuit", default)]
    circuit: Option<CircuitRaw>,
    #[serde(default)]
    date: String,
    #[serde(default)]
    time: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CircuitRaw {
    #[serde(rename = "circuitName", default)]
    circuit_name: String,
}

#[derive(Debug, Deserialize)]
struct StandingsResponse {
    #[serde(rename = "MRData")]
    mrdata: StandingsMRData,
}

#[derive(Debug, Deserialize)]
struct StandingsMRData {
    #[serde(rename = "StandingsTable")]
    standings_table: StandingsTable,
}

#[derive(Debug, Deserialize)]
struct StandingsTable {
    #[serde(rename = "StandingsLists", default)]
    standings_lists: Vec<StandingsList>,
}

#[derive(Debug, Deserialize)]
struct StandingsList {
    #[serde(rename = "DriverStandings", default)]
    driver_standings: Vec<DriverStandingRaw>,
}

#[derive(Debug, Deserialize)]
struct DriverStandingRaw {
    #[serde(default)]
    position: String,
    #[serde(default)]
    points: String,
    #[serde(rename = "Driver")]
    driver: DriverRaw,
}

#[derive(Debug, Deserialize)]
struct DriverRaw {
    #[serde(default)]
    code: String,
    #[serde(rename = "familyName", default)]
    family_name: String,
}

fn normalize(next: RaceTableResponse, stand: StandingsResponse) -> F1Data {
    let season = next.mrdata.race_table.season.clone();
    let next_race = next.mrdata.race_table.races.into_iter().next().and_then(|r| {
        let dt_str = match &r.time {
            Some(t) if !t.is_empty() => format!("{}T{}", r.date, t.trim_end_matches('Z')),
            _ => format!("{}T00:00:00", r.date),
        };
        let naive = NaiveDateTime::parse_from_str(&dt_str, "%Y-%m-%dT%H:%M:%S").ok()?;
        let start = Utc.from_utc_datetime(&naive).with_timezone(&Local);
        Some(NextRace {
            round: r.round.parse().unwrap_or(0),
            name: r.race_name,
            circuit: r.circuit.map(|c| c.circuit_name).unwrap_or_default(),
            start,
        })
    });

    let standings = stand
        .mrdata
        .standings_table
        .standings_lists
        .into_iter()
        .next()
        .map(|l| {
            l.driver_standings
                .into_iter()
                .map(|d| DriverStanding {
                    position: d.position.parse().unwrap_or(0),
                    code: if d.driver.code.is_empty() {
                        d.driver.family_name.chars().take(3).collect::<String>().to_uppercase()
                    } else {
                        d.driver.code
                    },
                    family_name: d.driver.family_name,
                    points: d.points.parse().unwrap_or(0.0),
                })
                .collect()
        })
        .unwrap_or_default();

    F1Data { season, next_race, standings }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NEXT_FIXTURE: &str = r#"
    {
      "MRData": {
        "RaceTable": {
          "season": "2026",
          "Races": [{
            "round": "5",
            "raceName": "Canadian Grand Prix",
            "Circuit": {"circuitName": "Circuit Gilles Villeneuve"},
            "date": "2026-05-24",
            "time": "20:00:00Z"
          }]
        }
      }
    }
    "#;

    const STAND_FIXTURE: &str = r#"
    {
      "MRData": {
        "StandingsTable": {
          "StandingsLists": [{
            "DriverStandings": [
              {"position": "1", "points": "100", "Driver": {"code": "ANT", "familyName": "Antonelli"}},
              {"position": "2", "points": "80",  "Driver": {"code": "RUS", "familyName": "Russell"}},
              {"position": "3", "points": "59",  "Driver": {"code": "LEC", "familyName": "Leclerc"}}
            ]
          }]
        }
      }
    }
    "#;

    #[test]
    fn parses_both_responses() {
        let n: RaceTableResponse = serde_json::from_str(NEXT_FIXTURE).unwrap();
        let s: StandingsResponse = serde_json::from_str(STAND_FIXTURE).unwrap();
        let d = normalize(n, s);
        assert_eq!(d.season, "2026");
        let nr = d.next_race.unwrap();
        assert_eq!(nr.round, 5);
        assert_eq!(nr.name, "Canadian Grand Prix");
        assert_eq!(nr.circuit, "Circuit Gilles Villeneuve");
        assert_eq!(d.standings.len(), 3);
        assert_eq!(d.standings[0].code, "ANT");
        assert_eq!(d.standings[0].points, 100.0);
    }

    #[test]
    fn missing_driver_code_falls_back_to_family_name_prefix() {
        let s: StandingsResponse = serde_json::from_str(
            r#"{"MRData":{"StandingsTable":{"StandingsLists":[{"DriverStandings":[
              {"position":"1","points":"25","Driver":{"familyName":"Hamilton"}}
            ]}]}}}"#,
        )
        .unwrap();
        let n: RaceTableResponse =
            serde_json::from_str(r#"{"MRData":{"RaceTable":{"Races":[]}}}"#).unwrap();
        let d = normalize(n, s);
        assert_eq!(d.standings[0].code, "HAM");
    }
}
