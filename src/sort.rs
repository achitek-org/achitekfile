//! Deterministic topological sorting for prompt dependency graphs.
//!
//! This module implements Kahn's algorithm for directed acyclic graphs (DAGs).
//! In this crate it is used to order prompts so that every prompt appears after
//! the prompts it depends on.
//!
//! Edges point from dependency to dependent:
//!
//! ```text
//! database -> orm
//! ```
//!
//! That edge means `orm` depends on `database`, so `database` must be sorted
//! first. Nodes that have no dependency relationship are emitted in the order
//! they appear in [`Graph::nodes`], giving callers deterministic output.
//!
//! Ported and modified from: <https://github.com/TheAlgorithms/Rust/blob/master/src/graph/topological_sort.rs>

use std::collections::{HashMap, VecDeque};

/// A directed graph represented as an adjacency list of edges.
///
/// Each edge is a tuple `(source, destination)`. For prompt ordering, `source`
/// is the referenced dependency prompt and `destination` is the prompt that
/// depends on it.
pub type DAGAsAdjacencyList<Node> = Vec<(Node, Node)>;

/// A graph data structure used for topological sorting.
///
/// `nodes` defines both the complete node set and the deterministic tie-breaker
/// order for independent nodes. `edges` defines ordering constraints between
/// those nodes.
#[derive(Debug, Clone)]
pub struct Graph<Node> {
    /// All nodes in the graph.
    pub nodes: Vec<Node>,
    /// Directed edges between nodes.
    pub edges: DAGAsAdjacencyList<Node>,
}

/// Sorts a graph with [Kahn's algorithm](https://en.wikipedia.org/wiki/Topological_sorting).
///
/// Given a graph, this function returns a vector of nodes in a valid
/// topological order. If the graph contains a cycle, it returns
/// [`SortError::CycleDetected`].
///
/// The implementation is deterministic: when multiple nodes are available to
/// emit, they are considered in the order they appear in [`Graph::nodes`].
///
/// # Examples
///
/// ```
/// use achitek::sort::{Graph, sort_graph};
///
/// let nodes: Vec<usize> = vec![2, 3, 5, 7, 8, 9, 10, 11];
/// let edges: Vec<(usize, usize)> = vec![
///     (5, 11),
///     (7, 8),
///     (7, 11),
///     (3, 8),
///     (3, 10),
///     (11, 2),
///     (11, 9),
///     (11, 10),
///     (8, 9),
/// ];
/// let graph: Graph<usize> = Graph { nodes, edges };
/// let sorted = sort_graph::<usize>(&graph);
///
/// assert!(sorted.is_ok());
/// ```
pub fn sort_graph<Node: std::hash::Hash + Eq + Clone>(
    graph: &Graph<Node>,
) -> Result<Vec<Node>, SortError<Node>> {
    // initialize data structures
    let mut dependencies_to_dependents_map: HashMap<Node, Vec<Node>> = HashMap::default();
    let mut in_degree_map: HashMap<Node, usize> = HashMap::default();
    // initialize the in-degree of all nodes to 0.
    for node in &graph.nodes {
        in_degree_map.entry(node.clone()).or_insert(0);
    }
    // build the dependency mapping and update in-degree counts based on graph edges.
    for (src, dest) in &graph.edges {
        dependencies_to_dependents_map
            .entry(src.clone())
            .or_default()
            .push(dest.clone());

        *in_degree_map.entry(dest.clone()).or_insert(0) += 1;
    }

    let mut queue: VecDeque<Node> = VecDeque::default();

    // add all nodes with zero in-degree to the queue in the graph's declared order.
    for node in &graph.nodes {
        if in_degree_map.get(node).is_some_and(|count| *count == 0) {
            queue.push_back(node.clone());
        }
    }

    let mut sorted: Vec<Node> = Vec::default();

    // process nodes from the queue, ensuring that dependencies are handled.
    while let Some(node_without_incoming_edges) = queue.pop_front() {
        sorted.push(node_without_incoming_edges.clone());

        in_degree_map.remove(&node_without_incoming_edges);

        // decrement the in-degree of each dependent node.
        for neighbor in dependencies_to_dependents_map
            .get(&node_without_incoming_edges)
            .unwrap_or(&vec![])
        {
            if let Some(count) = in_degree_map.get_mut(neighbor) {
                *count -= 1;

                // remove from in-degree map and add it to the queue if count becomes 0
                if *count == 0 {
                    in_degree_map.remove(neighbor);

                    queue.push_back(neighbor.clone());
                }
            }
        }
    }

    if in_degree_map.is_empty() {
        Ok(sorted)
    } else {
        Err(SortError::CycleDetected(graph.edges.clone()))
    }
}

/// Errors that can occur during topological sorting.
#[derive(Debug, Eq, PartialEq)]
pub enum SortError<Node> {
    /// A cycle was detected in the graph, making topological sorting impossible.
    ///
    /// Contains a vector of edges `(source, destination)` that form or are part of the cycle.
    /// When a cycle exists, there is no valid topological ordering of the nodes.
    CycleDetected(Vec<(Node, Node)>),
}

impl<Node> std::error::Error for SortError<Node> where
    Node: Clone + Ord + core::fmt::Display + core::fmt::Debug
{
}

impl<Node: Clone + Ord + std::fmt::Display + std::fmt::Debug> std::fmt::Display
    for SortError<Node>
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SortError::CycleDetected(edges) => {
                writeln!(f, "Cycle detected in the following DAG:")?;
                // Gather all unique nodes using Clone.
                let mut unique_nodes = std::collections::BTreeSet::new();
                for (src, dest) in edges.iter() {
                    unique_nodes.insert(src.clone());
                    unique_nodes.insert(dest.clone());
                }
                // Collect the unique nodes into a sorted vector.
                let sorted_nodes: Vec<Node> = unique_nodes.into_iter().collect();
                // Display the sorted nodes.
                writeln!(f, "Nodes:")?;
                for node in &sorted_nodes {
                    write!(f, "{} ", node)?;
                }
                writeln!(f, "\n")?;
                // Display edges with an arrow based on the order.
                writeln!(f, "Edges:")?;
                for (src, dest) in edges.iter() {
                    if src < dest {
                        writeln!(f, "  {} → {}", src, dest)?;
                    } else {
                        writeln!(f, "  {} ↖ {}", src, dest)?;
                    }
                }
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sort_graph_is_ok_integer() {
        let nodes: Vec<usize> = vec![2, 3, 5, 7, 8, 9, 10, 11];
        let edges: Vec<(usize, usize)> = vec![
            (5, 11),
            (7, 8),
            (7, 11),
            (3, 8),
            (3, 10),
            (11, 2),
            (11, 9),
            (11, 10),
            (8, 9),
        ];
        let graph: Graph<usize> = Graph { nodes, edges };
        let sorted = sort_graph::<usize>(&graph);

        assert!(sorted.is_ok());
    }

    #[test]
    fn test_sort_graph_is_err_integer() {
        let nodes: Vec<usize> = vec![2, 3, 5, 7, 8, 9, 10, 11];
        let edges: Vec<(usize, usize)> = vec![
            (5, 11),
            (7, 8),
            (7, 11),
            (3, 8),
            (3, 10),
            (11, 2),
            (11, 9),
            (11, 10),
            (8, 9),
            (9, 11), // <-- cycle introduced
        ];
        let graph: Graph<usize> = Graph { nodes, edges };
        let sorted = sort_graph::<usize>(&graph);

        assert!(sorted.is_err());
    }

    #[test]
    fn test_sort_graph_is_ok_strings() {
        let nodes = vec![
            "shirt",
            "hoodie",
            "socks",
            "underwear",
            "pants",
            "shoes",
            "glasses",
            "watch",
            "school",
        ];
        let edges = vec![
            ("shirt", "hoodie"),
            ("hoodie", "school"),
            ("underwear", "pants"),
            ("pants", "shoes"),
            ("socks", "shoes"),
            ("shoes", "school"),
        ];
        let graph: Graph<&str> = Graph { nodes, edges };
        let sorted = sort_graph::<&str>(&graph);

        assert!(sorted.is_ok());
    }

    #[test]
    fn test_sort_graph_keeps_node_order_when_independent() {
        let graph = Graph {
            nodes: vec!["first", "second", "third"],
            edges: Vec::new(),
        };

        assert_eq!(
            sort_graph::<&str>(&graph).expect("Expected graph to sort"),
            vec!["first", "second", "third"]
        );
    }

    #[test]
    fn test_is_err_strings() {
        let nodes = vec![
            "shirt",
            "hoodie",
            "socks",
            "underwear",
            "pants",
            "shoes",
            "glasses",
            "watch",
            "school",
        ];
        let edges = vec![
            ("shirt", "hoodie"),
            ("hoodie", "school"),
            ("school", "shirt"), // <-- cycle introduced
            ("underwear", "pants"),
            ("pants", "shoes"),
            ("socks", "shoes"),
            ("shoes", "school"),
        ];
        let graph: Graph<&str> = Graph { nodes, edges };
        let sorted = sort_graph::<&str>(&graph);

        assert!(sorted.is_err());
    }
}
