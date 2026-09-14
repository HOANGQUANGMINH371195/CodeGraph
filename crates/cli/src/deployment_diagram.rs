//! One-to-one declared graph projection. Rendering and Git verification are
//! separate gates; no source link or execution permission is invented here.
use graph_protocol::{SCHEMA_VERSION, archify as ir, deployment as wire};
use graph_store::Store;
use std::{collections::HashMap, error::Error};

use crate::deployment_context::ContextOptions;

pub fn query(store: &Store, options: &ContextOptions<'_>) -> Result<Vec<u8>, Box<dyn Error>> {
    if options.max_nodes > 12 || options.max_edges > 24 {
        return Err(
            std::io::Error::other("diagram overview requires <=12 nodes and <=24 edges").into(),
        );
    }
    let context = crate::deployment_context::report(store, options)?;
    let output = project(context)?;
    Ok(crate::output::json_line(&output, options.max_output_bytes)?)
}

fn project(context: wire::DeploymentContext) -> Result<ir::DeploymentDiagram, Box<dyn Error>> {
    let ids: HashMap<_, _> = context
        .nodes
        .iter()
        .enumerate()
        .map(|(i, node)| (node.id.as_str(), format!("n{i}")))
        .collect();
    let components = context
        .nodes
        .iter()
        .enumerate()
        .map(|(i, node)| ir::Component {
            id: format!("n{i}"),
            kind: ir::ComponentType::External,
            label: short_label(&node.name),
            sublabel: match node.kind {
                wire::DeploymentNodeKind::Service => "service",
                wire::DeploymentNodeKind::Volume => "volume",
                wire::DeploymentNodeKind::Network => "network",
            }
            .into(),
            tag: "declared".into(),
            row: i / 3,
            col: i % 3,
            size: [240, 80],
        })
        .collect();
    let connections = context
        .edges
        .iter()
        .enumerate()
        .map(|(i, edge)| {
            let endpoint = |id: &str| {
                ids.get(id)
                    .cloned()
                    .ok_or_else(|| std::io::Error::other("diagram context has a missing endpoint"))
            };
            Ok(ir::Connection {
                id: format!("e{i}"),
                from: endpoint(&edge.source)?,
                to: endpoint(&edge.target)?,
                label: match edge.kind {
                    wire::DeploymentEdgeKind::Mounts => "mounts",
                    wire::DeploymentEdgeKind::DependsOn => "depends_on",
                    wire::DeploymentEdgeKind::AttachedTo => "attached_to",
                }
                .into(),
                variant: ir::Variant::Dashed,
            })
        })
        .collect::<Result<Vec<_>, std::io::Error>>()?;
    let component_bindings = (0..context.nodes.len())
        .map(|i| ir::ComponentBinding {
            diagram_id: format!("n{i}"),
            context_node_index: i,
        })
        .collect();
    let connection_bindings = (0..context.edges.len())
        .map(|i| ir::ConnectionBinding {
            diagram_id: format!("e{i}"),
            context_edge_index: i,
        })
        .collect();
    let cards = evidence_cards(&context);
    Ok(ir::DeploymentDiagram {
        schema_version: SCHEMA_VERSION,
        kind: "deployment_diagram",
        diagram: ir::Architecture {
            schema_version: 1,
            diagram_type: ir::DiagramType::Architecture,
            meta: ir::Meta {
                title: "Declared deployment neighborhood".into(),
                subtitle: "Historical IaC facts; unknowns remain unresolved".into(),
                animation: ir::Animation::None,
                legend: ir::Legend {
                    mode: ir::LegendMode::Auto,
                    entries: ir::LegendEntries {
                        external: ir::LegendEntry {
                            label: "Declared service / volume / network".into(),
                        },
                    },
                },
            },
            layout: ir::Grid {
                mode: ir::LayoutMode::Grid,
                cols: 3,
                cell_w: 240,
                cell_h: 80,
                gap_x: 120,
                gap_y: 100,
            },
            components,
            connections,
            cards,
        },
        evidence_manifest: context,
        component_bindings,
        connection_bindings,
    })
}

fn evidence_cards(context: &wire::DeploymentContext) -> Vec<ir::Card> {
    vec![
        ir::Card {
            dot: ir::Dot::Amber,
            title: "Evidence and meaning".into(),
            items: vec![
                "Dashed edges: IaC declarations, not observed runtime traffic.".into(),
                "Historical snapshot; source bytes and analysis run are not verified here.".into(),
                "Full names, mount targets and citations are in the evidence manifest.".into(),
                "No verified Git links; diagram IDs are local to this bundle.".into(),
            ],
        },
        ir::Card {
            dot: ir::Dot::Slate,
            title: "Coverage and unknowns".into(),
            items: vec![
                format!(
                    "Generation {}. {} of {} nodes; {} of {} relationships.",
                    context.generation,
                    context.nodes.len(),
                    context.total_nodes,
                    context.edges.len(),
                    context.total_edges
                ),
                format!(
                    "Omitted: {} nodes, {} relationships (including outside this neighborhood).",
                    context.omitted_nodes, context.omitted_edges
                ),
                format!(
                    "Unknowns: {} across the entire owner, not attributed to this seed.",
                    context.unknown_count
                ),
                format!(
                    "Depth limited: {}; budget limited: {}.",
                    context.depth_limited, context.budget_limited
                ),
            ],
        },
    ]
}

fn short_label(name: &str) -> String {
    let mut chars = name.chars();
    let mut label: String = chars.by_ref().take(32).collect();
    if chars.next().is_some() {
        label.push('…');
    }
    label
}

#[cfg(test)]
mod tests {
    use super::short_label;

    #[test]
    fn shortening_preserves_unicode_boundaries_and_marks_only_actual_omissions() {
        assert_eq!(short_label("orders"), "orders");
        assert_eq!(short_label(&"界".repeat(32)), "界".repeat(32));
        assert_eq!(
            short_label(&"界".repeat(33)),
            format!("{}…", "界".repeat(32))
        );
    }
}
