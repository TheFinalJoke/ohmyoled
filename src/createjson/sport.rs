use crate::createjson::tui::field::{FieldDef, FieldKind};
use oledlib::teams;
use serde_json::Value;

/// TUI form schema. `team_logo` is a `ValueEnum` whose options depend on the
/// chosen `sport` enum — `form_module::on_field_changed` repopulates it when
/// the sport changes, and `team_options` builds the list.
pub fn fields() -> Vec<FieldDef> {
    vec![
        FieldDef::new(
            "sport",
            "League",
            "Which team sport.",
            FieldKind::Enum {
                default: "baseball",
                choices: &[
                    ("baseball", "MLB"),
                    ("basketball", "NBA"),
                    ("football", "NFL"),
                    ("hockey", "NHL"),
                ],
            },
        ),
        FieldDef::new(
            "team_logo",
            "Team",
            "Which team's scoreboard + standings to follow.",
            FieldKind::ValueEnum,
        ),
        FieldDef::new(
            "show_offseason",
            "Show offseason",
            "Draw a team-crest \"OFFSEASON\" card between seasons instead of skipping the slot.",
            FieldKind::Bool { default: false },
        ),
        FieldDef::new(
            "cache_ttl_secs",
            "Cache TTL (secs)",
            super::CACHE_TTL_HELP,
            FieldKind::CacheTtl,
        ),
    ]
}

/// Build the `(team name, serialized Logo)` option list for a sport slug,
/// sorted alphabetically for a stable picker. Unknown slug ⇒ empty.
pub fn team_options(sport_slug: &str) -> Vec<(String, Value)> {
    let built = match sport_slug {
        "baseball" => teams::Sport::build_baseball(),
        "basketball" => teams::Sport::build_basketball(),
        "football" => teams::Sport::build_football(),
        "hockey" => teams::Sport::build_hockey(),
        _ => return Vec::new(),
    };
    let mut opts: Vec<(String, Value)> = built
        .teams
        .into_iter()
        .map(|(name, logo)| (name, serde_json::to_value(logo).expect("Logo serializes")))
        .collect();
    opts.sort_by(|a, b| a.0.cmp(&b.0));
    opts
}

/// Index of a team in `options` by name (for picking a sensible default like
/// the Chicago Cubs), or 0 if not found.
pub fn default_team_index(options: &[(String, Value)], name: &str) -> usize {
    options.iter().position(|(n, _)| n == name).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use crate::createjson::tui::form_module;

    #[test]
    fn default_sport_picks_a_team() {
        let form = form_module::default_form("sport");
        let v = form_module::section_to_value("sport", &form).unwrap();
        assert_eq!(v["sport"], serde_json::json!("baseball"));
        assert!(v["team_logo"]["name"].is_string());
    }

    #[test]
    fn changing_sport_repopulates_teams() {
        let mut form = form_module::default_form("sport");
        // switch to basketball (index 1)
        if let crate::createjson::tui::field::FieldValue::Enum(sel) = &mut form.values[0] {
            *sel = 1;
        }
        form_module::on_field_changed("sport", &mut form, "sport");
        let v = form_module::section_to_value("sport", &form).unwrap();
        assert_eq!(v["sport"], serde_json::json!("basketball"));
        // the chosen team's logo carries the basketball sport tag
        assert_eq!(v["team_logo"]["sport"], serde_json::json!("basketball"));
    }
}
