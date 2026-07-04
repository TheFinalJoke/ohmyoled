use crate::createjson::tui::field::{FieldDef, FieldKind};

/// TUI form schema. The constant `sport: "f1"` is injected in
/// `form_module::section_to_value`. F1 needs no other config.
pub fn fields() -> Vec<FieldDef> {
    vec![
        FieldDef::new(
            "show_offseason",
            "Show offseason",
            "Draw the reigning-champion / \"OFFSEASON\" card between seasons instead of skipping the slot.",
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
