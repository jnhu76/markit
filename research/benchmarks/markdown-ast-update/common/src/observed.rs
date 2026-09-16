//! The three-valued observation type required by R0 counter semantics.
//!
//! Plain `Option<u64>` cannot distinguish "measured zero" from "the
//! mechanism never reported this" from "this counter does not apply to the
//! mechanism at all". R1 preserves all three states end to end.

use schemars::generate::SchemaGenerator;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::Serializer;
use std::borrow::Cow;

/// A work-counter / metric observation.
///
/// JSON representation (stable, schema-checked):
///
/// - `Known(v)`  -> the number itself
/// - `Unknown`   -> the string `"UNKNOWN"`
/// - `NotApplicable` -> the string `"NOT_APPLICABLE"`
///
/// `Known(0)` is therefore never conflated with `Unknown` or
/// `NotApplicable`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Observed<T> {
    Known(T),
    Unknown,
    NotApplicable,
}

pub const UNKNOWN_MARKER: &str = "UNKNOWN";
pub const NOT_APPLICABLE_MARKER: &str = "NOT_APPLICABLE";

impl<T> Observed<T> {
    pub fn is_known(&self) -> bool {
        matches!(self, Observed::Known(_))
    }

    pub fn known_value(&self) -> Option<T>
    where
        T: Copy,
    {
        match self {
            Observed::Known(v) => Some(*v),
            _ => None,
        }
    }
}

impl<T: Serialize> Serialize for Observed<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Observed::Known(v) => v.serialize(serializer),
            Observed::Unknown => serializer.serialize_str(UNKNOWN_MARKER),
            Observed::NotApplicable => serializer.serialize_str(NOT_APPLICABLE_MARKER),
        }
    }
}

/// JSON Schema mirrors the serde representation exactly:
/// `anyOf: [<T>, "UNKNOWN", "NOT_APPLICABLE"]`.
impl<T: JsonSchema> JsonSchema for Observed<T> {
    fn schema_name() -> Cow<'static, str> {
        Cow::Owned(format!("observed_{}", T::schema_name()))
    }

    fn schema_id() -> Cow<'static, str> {
        Cow::Owned(format!("[observed:{}]", T::schema_id()))
    }

    fn json_schema(generator: &mut SchemaGenerator) -> schemars::Schema {
        let value = serde_json::json!({
            "anyOf": [
                generator.subschema_for::<T>(),
                { "const": UNKNOWN_MARKER },
                { "const": NOT_APPLICABLE_MARKER },
            ]
        });
        serde_json::from_value(value).expect("observed schema is valid JSON Schema")
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Observed<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw<T> {
            Value(T),
            Marker(String),
        }
        match Raw::<T>::deserialize(deserializer)? {
            Raw::Value(v) => Ok(Observed::Known(v)),
            Raw::Marker(s) => match s.as_str() {
                UNKNOWN_MARKER => Ok(Observed::Unknown),
                NOT_APPLICABLE_MARKER => Ok(Observed::NotApplicable),
                other => Err(serde::de::Error::custom(format!(
                    "invalid observed marker: {other:?} (expected {UNKNOWN_MARKER} or {NOT_APPLICABLE_MARKER})"
                ))),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_zero_is_distinct_from_unknown_and_not_applicable() {
        assert_ne!(Observed::<u64>::Known(0), Observed::Unknown);
        assert_ne!(Observed::<u64>::Known(0), Observed::NotApplicable);
        assert_ne!(Observed::<u64>::Unknown, Observed::NotApplicable);
    }

    #[test]
    fn json_round_trip_preserves_all_three_states() {
        let vals = [
            Observed::<u64>::Known(0),
            Observed::Known(42),
            Observed::Unknown,
            Observed::NotApplicable,
        ];
        for v in vals {
            let json = serde_json::to_value(v).expect("serialize");
            let back: Observed<u64> = serde_json::from_value(json).expect("deserialize");
            assert_eq!(back, v);
        }
        assert_eq!(
            serde_json::to_value(Observed::Known(0)).unwrap(),
            serde_json::json!(0)
        );
        assert_eq!(
            serde_json::to_value(Observed::<u64>::Unknown).unwrap(),
            serde_json::json!("UNKNOWN")
        );
        assert_eq!(
            serde_json::to_value(Observed::<u64>::NotApplicable).unwrap(),
            serde_json::json!("NOT_APPLICABLE")
        );
    }

    #[test]
    fn invalid_marker_is_rejected() {
        let res: Result<Observed<u64>, _> = serde_json::from_value(serde_json::json!("NOT_KNOWN"));
        assert!(res.is_err());
    }
}
