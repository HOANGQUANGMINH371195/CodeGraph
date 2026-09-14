use crate::{ProtocolError, SCHEMA_VERSION, require_current_schema};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequiredChecks {
    pub schema_version: u32,
    pub names: Vec<String>,
}

impl RequiredChecks {
    pub fn try_into_domain(self) -> Result<graph_domain::RequiredChecks, ProtocolError> {
        require_current_schema(self.schema_version)?;
        Ok(graph_domain::RequiredChecks::new(self.names)?)
    }
}

impl From<&graph_domain::RequiredChecks> for RequiredChecks {
    fn from(policy: &graph_domain::RequiredChecks) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            names: policy.names().map(str::to_owned).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn strict_versioned_policy_normalizes_order_without_hiding_duplicates() {
        for raw in [
            r#"{"schema_version":1,"names":[],"trust":true}"#,
            r#"{"schema_version":1}"#,
        ] {
            assert!(serde_json::from_str::<RequiredChecks>(raw).is_err());
        }
        for (version, names) in [
            (2, vec!["test".into()]),
            (1, vec![]),
            (1, vec!["test".into(), "test".into()]),
            (1, vec![" ".into()]),
        ] {
            assert!(
                RequiredChecks {
                    schema_version: version,
                    names
                }
                .try_into_domain()
                .is_err()
            );
        }
        let domain = RequiredChecks {
            schema_version: 1,
            names: vec!["test".into(), "lint".into()],
        }
        .try_into_domain()
        .unwrap();
        let wire = RequiredChecks::from(&domain);
        assert_eq!(wire.names, vec!["lint", "test"]);
        assert_eq!(wire.try_into_domain().unwrap(), domain);
    }
}
