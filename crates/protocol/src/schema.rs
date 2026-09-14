//! Wire-schema compatibility policy.
//!
//! Protocol schema versions are separate from storage migrations, graph
//! generations, extractor versions and cache formats.  The current reader is
//! strict about its known fields; forward evolution happens through an
//! explicit, bounded extension object rather than silently accepting typos.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ProtocolError;

/// Current version of the ordinary JSON protocol DTOs.
pub const SCHEMA_VERSION: u32 = 1;

/// How a wire version relates to the current reader.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SchemaCompatibility {
    /// The version is understood by this reader.
    Current,
    /// The sender is older than this reader and needs an explicit adapter.
    Legacy(u32),
    /// The sender may contain fields/semantics this reader cannot understand.
    Future(u32),
}

/// Classify a wire version without silently upgrading or downgrading it.
#[must_use]
pub const fn schema_compatibility(version: u32) -> SchemaCompatibility {
    if version < SCHEMA_VERSION {
        SchemaCompatibility::Legacy(version)
    } else if version == SCHEMA_VERSION {
        SchemaCompatibility::Current
    } else {
        SchemaCompatibility::Future(version)
    }
}

/// Require the exact schema understood by the current DTO converter.
pub fn require_current_schema(version: u32) -> Result<(), ProtocolError> {
    if matches!(schema_compatibility(version), SchemaCompatibility::Current) {
        Ok(())
    } else {
        Err(ProtocolError::UnsupportedSchema(version))
    }
}

const MAX_EXTENSION_FIELDS: usize = 64;
const MAX_EXTENSION_KEY_BYTES: usize = 128;
const MAX_EXTENSION_VALUE_BYTES: usize = 16 * 1024;
const MAX_EXTENSION_TOTAL_BYTES: usize = 256 * 1024;

/// Explicit forward-compatible fields for DTOs that opt into extensions.
///
/// Keys must use `namespace/name` form. Values are untrusted data and are
/// bounded by their serialized JSON representation; this type does not make
/// them semantic facts or authority-bearing fields.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ExtensionFields(BTreeMap<String, serde_json::Value>);

impl ExtensionFields {
    /// Validate and wrap extension fields.
    pub fn new(fields: BTreeMap<String, serde_json::Value>) -> Result<Self, ProtocolError> {
        let extensions = Self(fields);
        extensions.validate()?;
        Ok(extensions)
    }

    /// Return an empty extension set.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Validate key namespaces and serialized value budgets.
    pub fn validate(&self) -> Result<(), ProtocolError> {
        if self.0.len() > MAX_EXTENSION_FIELDS {
            return Err(ProtocolError::InvalidSchemaExtensions(
                "too many extension fields",
            ));
        }
        let mut total = 0usize;
        for (key, value) in &self.0 {
            if !valid_extension_key(key) {
                return Err(ProtocolError::InvalidSchemaExtensions(
                    "extension key must be namespaced",
                ));
            }
            let value_bytes = serde_json::to_vec(value).map_err(|_| {
                ProtocolError::InvalidSchemaExtensions("extension value is not serializable")
            })?;
            if value_bytes.len() > MAX_EXTENSION_VALUE_BYTES {
                return Err(ProtocolError::InvalidSchemaExtensions(
                    "extension value exceeds byte budget",
                ));
            }
            total = total
                .saturating_add(key.len())
                .saturating_add(value_bytes.len());
            if total > MAX_EXTENSION_TOTAL_BYTES {
                return Err(ProtocolError::InvalidSchemaExtensions(
                    "extension fields exceed byte budget",
                ));
            }
        }
        Ok(())
    }

    /// Borrow the validated extension map.
    #[must_use]
    pub fn as_map(&self) -> &BTreeMap<String, serde_json::Value> {
        &self.0
    }
}

fn valid_extension_key(key: &str) -> bool {
    if key.is_empty() || key.len() > MAX_EXTENSION_KEY_BYTES || key.chars().any(char::is_control) {
        return false;
    }
    let Some((namespace, name)) = key.split_once('/') else {
        return false;
    };
    !namespace.is_empty()
        && !name.is_empty()
        && !name.contains('/')
        && namespace.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte)
        })
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_current_legacy_and_future_without_downgrade() {
        assert_eq!(
            schema_compatibility(SCHEMA_VERSION),
            SchemaCompatibility::Current
        );
        assert_eq!(schema_compatibility(0), SchemaCompatibility::Legacy(0));
        assert_eq!(
            schema_compatibility(SCHEMA_VERSION + 1),
            SchemaCompatibility::Future(SCHEMA_VERSION + 1)
        );
        assert!(require_current_schema(SCHEMA_VERSION).is_ok());
        assert!(require_current_schema(SCHEMA_VERSION + 1).is_err());
    }

    #[test]
    fn extensions_are_bounded_namespaced_and_round_trip() {
        let mut fields = BTreeMap::new();
        fields.insert("acme/trace-id".into(), serde_json::json!({"v": 1}));
        let extensions = ExtensionFields::new(fields).unwrap();
        let json = serde_json::to_string(&extensions).unwrap();
        let decoded: ExtensionFields = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, extensions);
        assert_eq!(decoded.as_map()["acme/trace-id"]["v"], 1);
    }

    #[test]
    fn extension_typos_and_oversized_values_fail_closed() {
        let mut bad_key = BTreeMap::new();
        bad_key.insert("trace-id".into(), serde_json::json!(true));
        assert!(ExtensionFields::new(bad_key).is_err());

        let mut bad_namespace = BTreeMap::new();
        bad_namespace.insert("Acme/value".into(), serde_json::json!(true));
        assert!(ExtensionFields::new(bad_namespace).is_err());

        let mut oversized = BTreeMap::new();
        oversized.insert("acme/value".into(), serde_json::json!("x".repeat(20_000)));
        assert!(ExtensionFields::new(oversized).is_err());
    }
}
