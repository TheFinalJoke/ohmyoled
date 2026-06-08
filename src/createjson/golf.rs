use crate::createjson::tui::field::{FieldDef, FieldKind};

/// TUI form schema. The constant `sport: "golf"` is injected in
/// `form_module::section_to_value`.
pub fn fields() -> Vec<FieldDef> {
    vec![
        FieldDef::new(
            "tour",
            "Tour",
            "Which golf tour leaderboard to follow.",
            FieldKind::Enum {
                default: "pga",
                choices: &[
                    ("pga", "PGA Tour"),
                    ("lpga", "LPGA Tour"),
                    ("champions", "PGA Champions Tour"),
                    ("korn", "Korn Ferry Tour"),
                    ("liv", "LIV Golf"),
                ],
            },
        ),
        FieldDef::new(
            "cache_ttl_secs",
            "Cache TTL (secs)",
            super::CACHE_TTL_HELP,
            FieldKind::CacheTtl,
        ),
    ]
}
