//! Source-verified Compose declarations, not runtime topology.

use std::collections::HashMap;

use graph_application::SourceSlice;
use graph_domain::SourceEvidence;
use sha2::{Digest, Sha256};

use crate::yaml::{Node, Value, YamlError, parse};

const MAX_SOURCE_BYTES: usize = 1024 * 1024;
type Entries = Vec<(String, u32, Node)>;
type Sections = HashMap<String, (u32, Node)>;

#[derive(Debug, thiserror::Error)]
pub enum ComposeError {
    #[error("invalid declared deployment graph: {0}")]
    InvalidGraph(#[from] graph_domain::DomainError),
    #[error("invalid Compose source: {reason} at line {line}")]
    InvalidSource { reason: &'static str, line: u32 },
    #[error("invalid Compose shape: {reason} at line {line}")]
    InvalidShape { reason: &'static str, line: u32 },
    #[error("invalid Compose YAML: {reason} at line {line}")]
    Yaml { reason: &'static str, line: u32 },
}

impl From<YamlError> for ComposeError {
    fn from(error: YamlError) -> Self {
        Self::Yaml {
            reason: error.reason,
            line: error.line,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum NodeKind {
    Service,
    Volume,
    Network,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeploymentNode {
    pub id: String,
    pub kind: NodeKind,
    pub name: String,
    pub evidence: SourceEvidence,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EdgeKind {
    Mounts,
    DependsOn,
    AttachedTo,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeploymentEdge {
    pub source: String,
    pub target: String,
    pub kind: EdgeKind,
    pub mount_target: Option<String>,
    pub evidence: SourceEvidence,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unknown {
    pub reason: &'static str,
    pub line: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComposeProjection {
    pub nodes: Vec<DeploymentNode>,
    pub edges: Vec<DeploymentEdge>,
    pub unknowns: Vec<Unknown>,
}

fn require_full_source(source: &SourceSlice) -> Result<(), ComposeError> {
    let evidence = source.evidence();
    if evidence.start_line() != 1 {
        return Err(ComposeError::InvalidSource {
            reason: "Compose analysis requires a full-file citation starting at line one",
            line: evidence.start_line(),
        });
    }
    if source.text().len() > MAX_SOURCE_BYTES {
        return Err(ComposeError::InvalidSource {
            reason: "source exceeds the one MiB Compose budget",
            line: evidence.start_line(),
        });
    }
    let hash = format!("{:x}", Sha256::digest(source.text().as_bytes()));
    if hash != evidence.content_sha256() {
        return Err(ComposeError::InvalidSource {
            reason: "verified slice is not the complete source named by its evidence",
            line: evidence.start_line(),
        });
    }
    Ok(())
}

/// Project explicitly declared Compose relationships, not observed runtime topology.
/// Unsupported semantics are retained as value-free diagnostics.
///
/// # Errors
/// Rejects partial or oversized source, unsupported YAML syntax/budgets and
/// malformed required structure. No partial projection is returned on error.
pub fn analyze_compose(source: &SourceSlice) -> Result<ComposeProjection, ComposeError> {
    require_full_source(source)?;
    let evidence = source.evidence();
    let mut unknowns = Vec::new();
    let sections = parse_sections(source, &mut unknowns)?;

    project_sections(source, evidence, &sections, unknowns)
}

fn parse_sections(
    source: &SourceSlice,
    unknowns: &mut Vec<Unknown>,
) -> Result<Sections, ComposeError> {
    let root = parse(source.text())?;
    let Value::Mapping(root_entries) = root.value else {
        return Err(ComposeError::InvalidShape {
            reason: "Compose document root",
            line: root.line,
        });
    };
    let mut sections = HashMap::new();
    for (key, line, node) in root_entries {
        match key.as_str() {
            "services" | "volumes" | "networks" => {
                sections.insert(key, (line, node));
            }
            _ => unknowns.push(Unknown {
                reason: "unsupported Compose top-level field",
                line,
            }),
        }
    }
    Ok(sections)
}

fn project_sections(
    source: &SourceSlice,
    evidence: &SourceEvidence,
    sections: &Sections,
    mut unknowns: Vec<Unknown>,
) -> Result<ComposeProjection, ComposeError> {
    let services = section(sections, "services", "services must be a mapping")?;
    let volumes = optional_section(sections, "volumes", "volumes must be a mapping")?;
    let networks = optional_section(sections, "networks", "networks must be a mapping")?;
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut ids = HashMap::new();

    for (resources, kind, reason) in [
        (volumes, NodeKind::Volume, "volume option is not modeled"),
        (networks, NodeKind::Network, "network option is not modeled"),
    ] {
        for (name, line, node) in resources.into_iter().flatten() {
            validate_name(name, *line)?;
            add_node(
                source,
                evidence,
                &mut nodes,
                &mut ids,
                kind.clone(),
                name,
                *line,
            )?;
            if !matches!(node.value, Value::Null | Value::Mapping(_)) {
                unknowns.push(Unknown {
                    reason: "unsupported resource declaration",
                    line: *line,
                });
            }
            resource_option_unknowns(node, reason, &mut unknowns);
        }
    }

    for (name, line, _node) in services {
        validate_name(name, *line)?;
        add_node(
            source,
            evidence,
            &mut nodes,
            &mut ids,
            NodeKind::Service,
            name,
            *line,
        )?;
    }
    for (name, line, node) in services {
        let fields = mapping(node, "service declaration")?;
        let mut has_networks = false;
        for (field, field_line, value) in fields {
            if scalar_contains_interpolation(value) {
                unknowns.push(Unknown {
                    reason: "interpolation not resolved",
                    line: *field_line,
                });
            }
            match field.as_str() {
                "volumes" => parse_volumes(evidence, name, value, &ids, &mut edges, &mut unknowns)?,
                "depends_on" => {
                    parse_depends_on(evidence, name, value, &ids, &mut edges, &mut unknowns)?;
                }
                "networks" => {
                    has_networks = true;
                    parse_networks(evidence, name, value, &ids, &mut edges, &mut unknowns)?;
                }
                "build" => unknowns.push(Unknown {
                    reason: "build is not modeled",
                    line: *field_line,
                }),
                "environment" => unknowns.push(Unknown {
                    reason: "environment is not modeled",
                    line: *field_line,
                }),
                "ports" => unknowns.push(Unknown {
                    reason: "ports are not modeled",
                    line: *field_line,
                }),
                _ => unknowns.push(Unknown {
                    reason: "unsupported service field",
                    line: *field_line,
                }),
            }
        }
        if !has_networks {
            unknowns.push(Unknown {
                reason: "implicit default network is not modeled",
                line: *line,
            });
        }
    }

    Ok(ComposeProjection {
        nodes,
        edges,
        unknowns,
    })
}

fn mapping<'a>(
    node: &'a Node,
    reason: &'static str,
) -> Result<&'a Vec<(String, u32, Node)>, ComposeError> {
    match &node.value {
        Value::Mapping(entries) => Ok(entries),
        _ => Err(ComposeError::InvalidShape {
            reason,
            line: node.line,
        }),
    }
}

fn section<'a>(
    sections: &'a HashMap<String, (u32, Node)>,
    name: &'static str,
    reason: &'static str,
) -> Result<&'a Vec<(String, u32, Node)>, ComposeError> {
    let Some((line, node)) = sections.get(name) else {
        return Err(ComposeError::InvalidShape {
            reason: "required services section is missing",
            line: 1,
        });
    };
    mapping(node, reason).map_err(|_| ComposeError::InvalidShape {
        reason,
        line: *line,
    })
}

fn optional_section<'a>(
    sections: &'a HashMap<String, (u32, Node)>,
    name: &'static str,
    reason: &'static str,
) -> Result<Option<&'a Entries>, ComposeError> {
    let Some((line, node)) = sections.get(name) else {
        return Ok(None);
    };
    Ok(Some(mapping(node, reason).map_err(|_| {
        ComposeError::InvalidShape {
            reason,
            line: *line,
        }
    })?))
}

fn validate_name(name: &str, line: u32) -> Result<(), ComposeError> {
    if name.is_empty() || name.contains(['\0', '\n', '\r']) {
        return Err(ComposeError::InvalidShape {
            reason: "declaration name is invalid",
            line,
        });
    }
    Ok(())
}

fn kind_name(kind: &NodeKind) -> &'static str {
    match kind {
        NodeKind::Service => "service",
        NodeKind::Volume => "volume",
        NodeKind::Network => "network",
    }
}

fn declaration_id(source: &SourceSlice, kind: &NodeKind, name: &str) -> String {
    let project = source.evidence().project();
    let digest = digest_components(&[
        &project.repository_id,
        &project.worktree_id,
        source.evidence().path(),
        kind_name(kind),
        name,
    ]);
    format!("compose:{}:{digest}", kind_name(kind))
}

fn digest_components(parts: &[&str]) -> String {
    let mut bytes = Vec::new();
    for part in parts {
        let length = u64::try_from(part.len()).unwrap_or(u64::MAX);
        bytes.extend_from_slice(&length.to_be_bytes());
        bytes.extend_from_slice(part.as_bytes());
    }
    format!("{:x}", Sha256::digest(bytes))
}

fn add_node(
    source: &SourceSlice,
    evidence: &SourceEvidence,
    nodes: &mut Vec<DeploymentNode>,
    ids: &mut HashMap<(NodeKind, String), String>,
    kind: NodeKind,
    name: &str,
    line: u32,
) -> Result<(), ComposeError> {
    let id = declaration_id(source, &kind, name);
    ids.insert((kind.clone(), name.to_owned()), id.clone());
    nodes.push(DeploymentNode {
        id,
        kind,
        name: name.to_owned(),
        evidence: evidence_at(evidence, line)?,
    });
    Ok(())
}

fn evidence_at(evidence: &SourceEvidence, line: u32) -> Result<SourceEvidence, ComposeError> {
    let line_text = line.to_string();
    let evidence_id = format!(
        "compose-evidence:{}",
        digest_components(&[
            evidence.project().repository_id.as_str(),
            evidence.project().worktree_id.as_str(),
            evidence.project().git_head.as_str(),
            evidence.project().working_tree_fingerprint.as_str(),
            evidence.project().config_hash.as_str(),
            evidence.project().ignore_policy_version.as_str(),
            evidence.graph_version(),
            evidence.path(),
            evidence.content_sha256(),
            line_text.as_str(),
            evidence.analysis_run(),
        ])
    );
    SourceEvidence::new(
        evidence_id,
        evidence.project().clone(),
        evidence.graph_version().to_owned(),
        evidence.path().to_owned(),
        evidence.content_sha256().to_owned(),
        line,
        line,
        evidence.analysis_run().to_owned(),
    )
    .map_err(|_| ComposeError::InvalidSource {
        reason: "source evidence could not be narrowed",
        line,
    })
}

fn long_mount(
    item: &Node,
    fields: &Entries,
    unknowns: &mut Vec<Unknown>,
) -> Option<(String, String, u32)> {
    let mut ty = None;
    let mut source_name = None;
    let mut target_name = None;
    let mut unsupported = false;
    for (key, key_line, value) in fields {
        match key.as_str() {
            "type" => ty = scalar_value(value),
            "source" => source_name = scalar_value(value),
            "target" => target_name = scalar_value(value),
            "read_only" => {
                if matches!(scalar_value(value).as_deref(), Some("true" | "false")) {
                    unknowns.push(Unknown {
                        reason: "mount access mode is not modeled",
                        line: *key_line,
                    });
                } else {
                    unsupported = true;
                    unknowns.push(Unknown {
                        reason: "volume option is not modeled",
                        line: *key_line,
                    });
                }
            }
            _ => {
                unsupported = true;
                unknowns.push(Unknown {
                    reason: "volume option is not modeled",
                    line: *key_line,
                });
            }
        }
    }
    if ty.as_deref() != Some("volume") || unsupported {
        unknowns.push(Unknown {
            reason: "long volume syntax is not modeled",
            line: item.line,
        });
        return None;
    }
    let (Some(source_name), Some(target_name)) = (source_name, target_name) else {
        unknowns.push(Unknown {
            reason: "long volume declaration is incomplete",
            line: item.line,
        });
        return None;
    };
    Some((source_name, target_name, item.line))
}

fn parse_volumes(
    evidence: &SourceEvidence,
    service: &str,
    node: &Node,
    ids: &HashMap<(NodeKind, String), String>,
    edges: &mut Vec<DeploymentEdge>,
    unknowns: &mut Vec<Unknown>,
) -> Result<(), ComposeError> {
    let Value::Sequence(items) = &node.value else {
        return Err(ComposeError::InvalidShape {
            reason: "service volumes must use a sequence",
            line: node.line,
        });
    };
    for item in items {
        let (source_name, target_name, line) = match &item.value {
            Value::Scalar(spec) => {
                let parts: Vec<_> = spec.split(':').collect();
                if parts.len() == 2 && parts.iter().all(|part| !part.is_empty()) {
                    (parts[0].to_owned(), parts[1].to_owned(), item.line)
                } else if parts.len() == 3 && !parts[0].is_empty() && !parts[1].is_empty() {
                    if matches!(parts[2], "ro" | "rw") {
                        unknowns.push(Unknown {
                            reason: "mount access mode is not modeled",
                            line: item.line,
                        });
                        (parts[0].to_owned(), parts[1].to_owned(), item.line)
                    } else {
                        unknowns.push(Unknown {
                            reason: "volume mount option is not modeled",
                            line: item.line,
                        });
                        continue;
                    }
                } else {
                    unknowns.push(Unknown {
                        reason: "volume declaration is not modeled",
                        line: item.line,
                    });
                    continue;
                }
            }
            Value::Mapping(fields) => {
                let Some(mount) = long_mount(item, fields, unknowns) else {
                    continue;
                };
                mount
            }
            _ => {
                unknowns.push(Unknown {
                    reason: "volume declaration is not modeled",
                    line: item.line,
                });
                continue;
            }
        };
        if source_name.contains('$') || target_name.contains('$') {
            unknowns.push(Unknown {
                reason: "interpolation not resolved",
                line,
            });
            continue;
        }
        if !target_name.starts_with('/') {
            unknowns.push(Unknown {
                reason: "volume target must be an absolute path",
                line,
            });
            continue;
        }
        let Some(target) = ids.get(&(NodeKind::Volume, source_name)) else {
            unknowns.push(Unknown {
                reason: "mount source is not a declared volume",
                line,
            });
            continue;
        };
        let Some(source_id) = ids.get(&(NodeKind::Service, service.to_owned())) else {
            return Err(ComposeError::InvalidShape {
                reason: "service declaration is not indexed",
                line,
            });
        };
        edges.push(DeploymentEdge {
            source: source_id.clone(),
            target: target.clone(),
            kind: EdgeKind::Mounts,
            mount_target: Some(target_name),
            evidence: evidence_at(evidence, line)?,
        });
    }
    Ok(())
}

fn parse_depends_on(
    evidence: &SourceEvidence,
    service: &str,
    node: &Node,
    ids: &HashMap<(NodeKind, String), String>,
    edges: &mut Vec<DeploymentEdge>,
    unknowns: &mut Vec<Unknown>,
) -> Result<(), ComposeError> {
    let names = match &node.value {
        Value::Sequence(items) => items
            .iter()
            .map(|item| scalar_name(item, "depends_on entry").map(|name| (name, item.line)))
            .collect::<Result<Vec<_>, _>>()?,
        Value::Mapping(items) => {
            for (_name, key_line, options) in items {
                if !empty_options(options) {
                    unknowns.push(Unknown {
                        reason: "depends_on options are not modeled",
                        line: *key_line,
                    });
                }
            }
            items
                .iter()
                .map(|(name, key_line, _)| Ok((name.clone(), *key_line)))
                .collect::<Result<Vec<_>, ComposeError>>()?
        }
        _ => {
            return Err(ComposeError::InvalidShape {
                reason: "depends_on must be a sequence or mapping",
                line: node.line,
            });
        }
    };
    for (name, item_line) in names {
        if name.contains('$') {
            unknowns.push(Unknown {
                reason: "interpolation not resolved",
                line: item_line,
            });
            continue;
        }
        let Some(target) = ids.get(&(NodeKind::Service, name)) else {
            unknowns.push(Unknown {
                reason: "depends_on service is not declared",
                line: item_line,
            });
            continue;
        };
        let Some(source_id) = ids.get(&(NodeKind::Service, service.to_owned())) else {
            return Err(ComposeError::InvalidShape {
                reason: "service declaration is not indexed",
                line: item_line,
            });
        };
        edges.push(DeploymentEdge {
            source: source_id.clone(),
            target: target.clone(),
            kind: EdgeKind::DependsOn,
            mount_target: None,
            evidence: evidence_at(evidence, item_line)?,
        });
    }
    Ok(())
}

fn parse_networks(
    evidence: &SourceEvidence,
    service: &str,
    node: &Node,
    ids: &HashMap<(NodeKind, String), String>,
    edges: &mut Vec<DeploymentEdge>,
    unknowns: &mut Vec<Unknown>,
) -> Result<(), ComposeError> {
    let names = match &node.value {
        Value::Sequence(items) => items
            .iter()
            .map(|item| scalar_name(item, "network entry").map(|name| (name, item.line)))
            .collect::<Result<Vec<_>, _>>()?,
        Value::Mapping(items) => {
            for (_name, key_line, options) in items {
                if !empty_options(options) {
                    unknowns.push(Unknown {
                        reason: "network options are not modeled",
                        line: *key_line,
                    });
                }
            }
            items
                .iter()
                .map(|(name, key_line, _)| Ok((name.clone(), *key_line)))
                .collect::<Result<Vec<_>, ComposeError>>()?
        }
        _ => {
            return Err(ComposeError::InvalidShape {
                reason: "service networks must be a sequence or mapping",
                line: node.line,
            });
        }
    };
    for (name, item_line) in names {
        if name.contains('$') {
            unknowns.push(Unknown {
                reason: "interpolation not resolved",
                line: item_line,
            });
            continue;
        }
        let Some(target) = ids.get(&(NodeKind::Network, name)) else {
            unknowns.push(Unknown {
                reason: "network attachment is not declared",
                line: item_line,
            });
            continue;
        };
        let Some(source_id) = ids.get(&(NodeKind::Service, service.to_owned())) else {
            return Err(ComposeError::InvalidShape {
                reason: "service declaration is not indexed",
                line: item_line,
            });
        };
        edges.push(DeploymentEdge {
            source: source_id.clone(),
            target: target.clone(),
            kind: EdgeKind::AttachedTo,
            mount_target: None,
            evidence: evidence_at(evidence, item_line)?,
        });
    }
    Ok(())
}

fn scalar_name(node: &Node, reason: &'static str) -> Result<String, ComposeError> {
    match &node.value {
        Value::Scalar(value) if !value.is_empty() => Ok(value.clone()),
        _ => Err(ComposeError::InvalidShape {
            reason,
            line: node.line,
        }),
    }
}

fn scalar_value(node: &Node) -> Option<String> {
    match &node.value {
        Value::Scalar(value) => Some(value.clone()),
        _ => None,
    }
}

fn resource_option_unknowns(node: &Node, reason: &'static str, unknowns: &mut Vec<Unknown>) {
    if let Value::Mapping(fields) = &node.value {
        for (_, line, _) in fields {
            unknowns.push(Unknown {
                reason,
                line: *line,
            });
        }
    }
}

fn empty_options(node: &Node) -> bool {
    matches!(&node.value, Value::Null)
        || matches!(&node.value, Value::Mapping(values) if values.is_empty())
}

fn scalar_contains_interpolation(node: &Node) -> bool {
    match &node.value {
        Value::Scalar(value) => value.contains('$'),
        Value::Sequence(items) => items.iter().any(scalar_contains_interpolation),
        Value::Mapping(items) => items
            .iter()
            .any(|(_, _, value)| scalar_contains_interpolation(value)),
        Value::Null => false,
    }
}
