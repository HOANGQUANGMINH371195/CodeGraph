//! Historical, direct selection from one code-to-SQL candidate owner. No I/O.

use graph_domain::{
    DomainError,
    sql_link::{SqlLink, SqlLinkGraph},
};

/// Select exactly one persisted candidate link. A link owner is one code file,
/// not a traversable runtime/data-flow graph; returning a BFS neighborhood
/// would imply relationships this aggregate does not establish.
///
/// # Errors
/// Returns an error when the requested candidate identity is not in the owner.
pub fn select<'graph>(
    graph: &'graph SqlLinkGraph,
    link_id: &str,
) -> Result<&'graph SqlLink, DomainError> {
    graph
        .links()
        .iter()
        .find(|link| link.id == link_id)
        .ok_or(DomainError::Invalid("SQL-link context seed not found"))
}
