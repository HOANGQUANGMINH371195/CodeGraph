//! Neutral architecture IR for exactly one static code-to-SQL candidate.

use std::{error::Error, path::Path};

use graph_protocol::{SCHEMA_VERSION, archify as ir, sql_link as wire};
use graph_store::Store;

pub fn query(
    store: &Store,
    id: &str,
    task_path: &Path,
    link_id: &str,
    cap: u32,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let context = crate::sql_link_context::report(store, id, task_path, link_id)?;
    let output = project(context);
    Ok(crate::output::json_line(&output, cap as usize)?)
}

fn project(context: wire::SqlLinkContext) -> ir::SqlLinkDiagram {
    let components = vec![
        component("source", &context.code_evidence.path, "code source", 0),
        component("target", &context.candidate_path, "SQL file candidate", 1),
    ];
    ir::SqlLinkDiagram {
        schema_version: SCHEMA_VERSION,
        kind: "sql_link_diagram",
        diagram: ir::Architecture {
            schema_version: 1,
            diagram_type: ir::DiagramType::Architecture,
            meta: ir::Meta {
                title: "Static code-to-SQL file candidate".into(),
                subtitle: "Historical file-read candidate; not runtime data flow".into(),
                animation: ir::Animation::None,
                legend: ir::Legend {
                    mode: ir::LegendMode::Auto,
                    entries: ir::LegendEntries {
                        external: ir::LegendEntry {
                            label: "Code source / SQL file candidate".into(),
                        },
                    },
                },
            },
            layout: ir::Grid {
                mode: ir::LayoutMode::Grid,
                cols: 2,
                cell_w: 280,
                cell_h: 88,
                gap_x: 140,
                gap_y: 100,
            },
            components,
            connections: vec![ir::Connection {
                id: "candidate".into(),
                from: "source".into(),
                to: "target".into(),
                label: "static file-read candidate".into(),
                variant: ir::Variant::Dashed,
            }],
            cards: evidence_cards(&context),
        },
        evidence_manifest: context,
        source_component_id: "source".into(),
        target_component_id: "target".into(),
        connection_id: "candidate".into(),
    }
}

fn component(id: &str, path: &str, sublabel: &str, col: usize) -> ir::Component {
    ir::Component {
        id: id.into(),
        kind: ir::ComponentType::External,
        label: short_label(path),
        sublabel: sublabel.into(),
        tag: "candidate_only".into(),
        row: 0,
        col,
        size: [280, 88],
    }
}

fn evidence_cards(context: &wire::SqlLinkContext) -> Vec<ir::Card> {
    vec![
        ir::Card {
            dot: ir::Dot::Amber,
            title: "Evidence and meaning".into(),
            items: vec![
                "Dashed connection: static file-read candidate, not runtime traffic.".into(),
                "Neither physical table/database identity nor authorization is inferred.".into(),
                "Full citations, UTF-16 extent and syntax observations are in the evidence manifest.".into(),
            ],
        },
        ir::Card {
            dot: ir::Dot::Slate,
            title: "Coverage and verification".into(),
            items: vec![
                format!(
                    "Generation {}. One of {} candidate links; {} omitted.",
                    context.generation, context.total_links, context.omitted_links
                ),
                format!(
                    "Selected syntax statements: {} of {}; {} omitted.",
                    context.statements.len(), context.total_statements, context.omitted_statements
                ),
                "Historical only; source, relationship, runtime and semantic verification are false.".into(),
            ],
        },
    ]
}

fn short_label(path: &str) -> String {
    let mut chars = path.chars();
    let mut label: String = chars.by_ref().take(48).collect();
    if chars.next().is_some() {
        label.push('…');
    }
    label
}

#[cfg(test)]
mod tests {
    use super::short_label;

    #[test]
    fn short_label_preserves_unicode_boundaries_and_marks_only_omission() {
        assert_eq!(short_label("sql/q.sql"), "sql/q.sql");
        assert_eq!(short_label(&"界".repeat(48)), "界".repeat(48));
        assert_eq!(
            short_label(&"界".repeat(49)),
            format!("{}…", "界".repeat(48))
        );
    }
}
