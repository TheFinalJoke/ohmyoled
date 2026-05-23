//! Custom serde helpers for the project's config shape.
//!
//! The config files in the wild store unset `Option<String>` fields as the
//! literal string `"null"` (left over from how the old `json`-crate path
//! serialised `None`). These deserializers preserve that quirk so existing
//! configs keep parsing.

use serde::{Deserialize, Deserializer};

/// `Option<String>` deserializer that maps `"null"` (the string) → `None`.
pub fn null_string_as_none<'de, D>(de: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(de)?;
    Ok(match opt {
        Some(s) if s == "null" || s.is_empty() => None,
        other => other,
    })
}

/// `Vec<T>` deserializer that also accepts a single `T` object.
///
/// Lets each config section be either `"sport": {...}` (single instance, the
/// legacy shape) or `"sport": [{...}, {...}]` (multiple instances in the same
/// rotation).
pub fn one_or_many<'de, T, D>(de: D) -> Result<Vec<T>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum OneOrMany<T> {
        One(T),
        Many(Vec<T>),
    }
    Ok(match OneOrMany::<T>::deserialize(de)? {
        OneOrMany::One(t) => vec![t],
        OneOrMany::Many(v) => v,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Deserialize, PartialEq, Eq)]
    struct Item {
        id: u32,
    }

    #[derive(Debug, Deserialize)]
    struct Wrap {
        #[serde(default, deserialize_with = "one_or_many")]
        items: Vec<Item>,
    }

    #[test]
    fn accepts_single_object() {
        let w: Wrap = serde_json::from_str(r#"{"items": {"id": 1}}"#).unwrap();
        assert_eq!(w.items, vec![Item { id: 1 }]);
    }

    #[test]
    fn accepts_array() {
        let w: Wrap = serde_json::from_str(r#"{"items": [{"id": 1}, {"id": 2}]}"#).unwrap();
        assert_eq!(w.items, vec![Item { id: 1 }, Item { id: 2 }]);
    }

    #[test]
    fn missing_defaults_to_empty() {
        let w: Wrap = serde_json::from_str("{}").unwrap();
        assert!(w.items.is_empty());
    }
}
