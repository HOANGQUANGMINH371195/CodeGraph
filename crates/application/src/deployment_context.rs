//! Deterministic bounded traversal of one validated declared graph. No I/O.
use graph_domain::{DomainError, deployment::DeploymentGraph};
use std::collections::{HashMap, VecDeque};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Outgoing,
    Incoming,
    Both,
}

#[derive(Clone, Copy, Debug)]
pub struct ContextLimits {
    depth: u32,
    nodes: usize,
    edges: usize,
}
impl ContextLimits {
    /// Bound graph traversal, independently of serialized-output byte limits.
    ///
    /// # Errors
    /// Rejects depth above 16, zero/over-1000 nodes, or more than 5000 edges.
    pub fn new(depth: u32, max_nodes: usize, max_edges: usize) -> Result<Self, DomainError> {
        if depth > 16 || max_nodes == 0 || max_nodes > 1000 || max_edges > 5000 {
            return Err(DomainError::Invalid(
                "context limits require depth <=16, 1..1000 nodes and <=5000 edges",
            ));
        }
        Ok(Self {
            depth,
            nodes: max_nodes,
            edges: max_edges,
        })
    }
}

/// Indices into the exact input graph, not independently verified capabilities.
#[derive(Debug, PartialEq, Eq)]
pub struct Selection {
    pub nodes: Vec<usize>,
    pub edges: Vec<usize>,
    pub depth_limited: bool,
    pub budget_limited: bool,
}

/// Select a connected neighborhood in deterministic BFS/edge insertion order.
/// Returned edges retain their original orientation, even for incoming walks.
///
/// # Errors
/// Rejects an unknown seed or an inconsistent input graph.
pub fn select(
    graph: &DeploymentGraph,
    seed: &str,
    direction: Direction,
    limits: ContextLimits,
) -> Result<Selection, DomainError> {
    let indices: HashMap<_, _> = graph
        .nodes()
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id.as_str(), i))
        .collect();
    let seed = *indices
        .get(seed)
        .ok_or(DomainError::Invalid("deployment context seed not found"))?;
    let mut adjacency = vec![Vec::new(); graph.nodes().len()];
    for (i, e) in graph.edges().iter().enumerate() {
        let source = *indices
            .get(e.source.as_str())
            .ok_or(DomainError::Invalid("missing deployment endpoint"))?;
        let target = *indices
            .get(e.target.as_str())
            .ok_or(DomainError::Invalid("missing deployment endpoint"))?;
        if direction != Direction::Incoming {
            adjacency[source].push((i, target));
        }
        if direction != Direction::Outgoing {
            adjacency[target].push((i, source));
        }
    }
    let mut selected = Selection {
        nodes: vec![seed],
        edges: Vec::new(),
        depth_limited: false,
        budget_limited: false,
    };
    let mut nodes_seen = vec![false; graph.nodes().len()];
    nodes_seen[seed] = true;
    let mut edges_seen = vec![false; graph.edges().len()];
    let mut queue = VecDeque::from([(seed, 0)]);
    while let Some((node, depth)) = queue.pop_front() {
        for &(edge, next) in &adjacency[node] {
            if edges_seen[edge] {
                continue;
            }
            if depth >= limits.depth {
                selected.depth_limited = true;
                continue;
            }
            if selected.edges.len() >= limits.edges
                || (!nodes_seen[next] && selected.nodes.len() >= limits.nodes)
            {
                selected.budget_limited = true;
                continue;
            }
            if !nodes_seen[next] {
                nodes_seen[next] = true;
                selected.nodes.push(next);
                queue.push_back((next, depth + 1));
            }
            edges_seen[edge] = true;
            selected.edges.push(edge);
        }
    }
    Ok(selected)
}
