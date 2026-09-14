//! Output-only subset of pinned Archify architecture IR and its separate
//! historical evidence manifest. This is not a renderer verification receipt.
use serde::Serialize;

use crate::{deployment::DeploymentContext, sql_link::SqlLinkContext};

#[derive(Debug, Serialize)]
pub struct DeploymentDiagram {
    pub schema_version: u32,
    pub kind: &'static str,
    pub diagram: Architecture,
    pub evidence_manifest: DeploymentContext,
    pub component_bindings: Vec<ComponentBinding>,
    pub connection_bindings: Vec<ConnectionBinding>,
}

/// A bounded visual projection of one static file-read candidate. It is not a
/// runtime/data-flow, database-instance, physical-table, or authorization fact.
#[derive(Debug, Serialize)]
pub struct SqlLinkDiagram {
    pub schema_version: u32,
    pub kind: &'static str,
    pub diagram: Architecture,
    pub evidence_manifest: SqlLinkContext,
    pub source_component_id: String,
    pub target_component_id: String,
    pub connection_id: String,
}

/// IDs are local to this bundle, not stable identities for revision comparison.
#[derive(Debug, Serialize)]
pub struct ComponentBinding {
    pub diagram_id: String,
    pub context_node_index: usize,
}

#[derive(Debug, Serialize)]
pub struct ConnectionBinding {
    pub diagram_id: String,
    pub context_edge_index: usize,
}

#[derive(Debug, Serialize)]
pub struct Architecture {
    pub schema_version: u32,
    pub diagram_type: DiagramType,
    pub meta: Meta,
    pub layout: Grid,
    pub components: Vec<Component>,
    pub connections: Vec<Connection>,
    pub cards: Vec<Card>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagramType {
    Architecture,
}

#[derive(Debug, Serialize)]
pub struct Meta {
    pub title: String,
    pub subtitle: String,
    pub animation: Animation,
    pub legend: Legend,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Animation {
    None,
}

#[derive(Debug, Serialize)]
pub struct Legend {
    pub mode: LegendMode,
    pub entries: LegendEntries,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LegendMode {
    Auto,
}

#[derive(Debug, Serialize)]
pub struct LegendEntries {
    pub external: LegendEntry,
}

#[derive(Debug, Serialize)]
pub struct LegendEntry {
    pub label: String,
}

#[derive(Debug, Serialize)]
pub struct Grid {
    pub mode: LayoutMode,
    pub cols: usize,
    #[serde(rename = "cellW")]
    pub cell_w: u32,
    #[serde(rename = "cellH")]
    pub cell_h: u32,
    #[serde(rename = "gapX")]
    pub gap_x: u32,
    #[serde(rename = "gapY")]
    pub gap_y: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LayoutMode {
    Grid,
}

#[derive(Debug, Serialize)]
pub struct Component {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: ComponentType,
    pub label: String,
    pub sublabel: String,
    pub tag: String,
    pub row: usize,
    pub col: usize,
    pub size: [u32; 2],
}

/// Neutral renderer category: never infer a database or cloud from Compose kind.
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ComponentType {
    External,
}

#[derive(Debug, Serialize)]
pub struct Connection {
    pub id: String,
    pub from: String,
    pub to: String,
    pub label: String,
    pub variant: Variant,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Variant {
    Dashed,
}

#[derive(Debug, Serialize)]
pub struct Card {
    pub dot: Dot,
    pub title: String,
    pub items: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Dot {
    Amber,
    Slate,
}
