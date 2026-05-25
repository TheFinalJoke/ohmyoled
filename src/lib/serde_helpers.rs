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
///
/// Implemented as a hand-rolled `Visitor` rather than `#[serde(untagged)]`
/// so the inner deserialize error (e.g. `unknown variant 'coingecko'`)
/// surfaces directly. The untagged-enum variant tries both shapes and on
/// failure reports a useless `data did not match any variant of untagged
/// enum OneOrMany`, which costs hours of bisecting whenever a config
/// section breaks.
pub fn one_or_many<'de, T, D>(de: D) -> Result<Vec<T>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    use serde::de::{value::MapAccessDeserializer, MapAccess, SeqAccess, Visitor};
    use std::fmt;
    use std::marker::PhantomData;

    struct OneOrManyVisitor<T>(PhantomData<fn() -> T>);

    impl<'de, T> Visitor<'de> for OneOrManyVisitor<T>
    where
        T: Deserialize<'de>,
    {
        type Value = Vec<T>;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("a single object or an array of objects")
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Vec<T>, A::Error> {
            let mut out = Vec::with_capacity(seq.size_hint().unwrap_or(0));
            // Manual loop (vs `Vec::<T>::deserialize`) preserves the
            // element index in the error path — `[2]` lands in the
            // message instead of being eaten by the inner Vec impl.
            while let Some(item) = seq.next_element::<T>()? {
                out.push(item);
            }
            Ok(out)
        }

        fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Vec<T>, A::Error> {
            // Single-object case — forward the same map to T so the
            // field-level error message (and `serde_path_to_error`
            // path, when wrapped) reads naturally.
            let item = T::deserialize(MapAccessDeserializer::new(map))?;
            Ok(vec![item])
        }
    }

    de.deserialize_any(OneOrManyVisitor(PhantomData))
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

    #[test]
    fn bad_element_surfaces_inner_error() {
        // Before the visitor rewrite this returned the useless
        // "data did not match any variant of untagged enum OneOrMany"
        // message. Now the underlying type error from T comes through.
        let err = serde_json::from_str::<Wrap>(r#"{"items": [{"id": "not-a-number"}]}"#).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("invalid type") || msg.contains("expected"),
            "expected a typed-deserialize error, got {msg}"
        );
        assert!(
            !msg.contains("untagged enum OneOrMany"),
            "OneOrMany wrapper should not appear in error: {msg}"
        );
    }

    #[test]
    fn bad_single_object_surfaces_inner_error() {
        // Same contract for the single-object path.
        let err = serde_json::from_str::<Wrap>(r#"{"items": {"id": "not-a-number"}}"#).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("invalid type") || msg.contains("expected"));
        assert!(!msg.contains("untagged enum OneOrMany"));
    }
}
