use crate::createjson::tui::field::{FieldDef, FieldKind};

/// TUI form schema. The constant `sport: "f1"` is injected in
/// `form_module::section_to_value`. F1 needs no other config.
pub fn fields() -> Vec<FieldDef> {
    vec![FieldDef::new(
        "cache_ttl_secs",
        "Cache TTL (secs)",
        super::CACHE_TTL_HELP,
        FieldKind::CacheTtl,
    )]
}
