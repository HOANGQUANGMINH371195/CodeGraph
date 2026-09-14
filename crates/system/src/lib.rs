//! Offline system-declaration adapters. No runtime, network or filesystem access.

mod compose;
mod deployment;
pub use deployment::analyze_compose_graph;
pub use graph_domain::deployment::DeploymentGraph;
mod sql;
mod yaml;
pub use compose::{
    ComposeError, ComposeProjection, DeploymentEdge, DeploymentNode, EdgeKind, NodeKind, Unknown,
    analyze_compose,
};
pub use sql::{SqlError, SqlOperation, SqlSpan, SqlStatement, SqlSyntaxReport, analyze_sqlite};
