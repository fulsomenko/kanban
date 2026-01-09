use std::collections::{HashMap, HashSet, VecDeque};
use uuid::Uuid;

/// Check if adding an edge would create a cycle in a directed graph
///
/// Uses DFS to detect if there's already a path from target to source.
/// If such a path exists, adding source->target would create a cycle.
pub fn would_create_cycle(adj_list: &HashMap<Uuid, Vec<Uuid>>, source: Uuid, target: Uuid) -> bool {
    // Check if there's a path from target to source
    // If yes, adding source->target creates a cycle
    has_path(adj_list, target, source)
}

/// Check if a path exists from start to end using DFS
fn has_path(adj_list: &HashMap<Uuid, Vec<Uuid>>, start: Uuid, end: Uuid) -> bool {
    if start == end {
        return true;
    }

    let mut visited = HashSet::new();
    let mut stack = vec![start];

    while let Some(node) = stack.pop() {
        if node == end {
            return true;
        }

        if visited.insert(node) {
            if let Some(neighbors) = adj_list.get(&node) {
                for &neighbor in neighbors {
                    if !visited.contains(&neighbor) {
                        stack.push(neighbor);
                    }
                }
            }
        }
    }

    false
}

/// Detect if the graph contains any cycles using DFS
pub fn has_cycle(adj_list: &HashMap<Uuid, Vec<Uuid>>) -> bool {
    let mut visited = HashSet::new();
    let mut rec_stack = HashSet::new();

    for &node in adj_list.keys() {
        if !visited.contains(&node) && has_cycle_util(adj_list, node, &mut visited, &mut rec_stack)
        {
            return true;
        }
    }

    false
}

fn has_cycle_util(
    adj_list: &HashMap<Uuid, Vec<Uuid>>,
    node: Uuid,
    visited: &mut HashSet<Uuid>,
    rec_stack: &mut HashSet<Uuid>,
) -> bool {
    visited.insert(node);
    rec_stack.insert(node);

    if let Some(neighbors) = adj_list.get(&node) {
        for &neighbor in neighbors {
            if !visited.contains(&neighbor) {
                if has_cycle_util(adj_list, neighbor, visited, rec_stack) {
                    return true;
                }
            } else if rec_stack.contains(&neighbor) {
                return true;
            }
        }
    }

    rec_stack.remove(&node);
    false
}

/// Get all nodes reachable from a given node (transitive closure) using BFS
pub fn reachable_from(adj_list: &HashMap<Uuid, Vec<Uuid>>, start: Uuid) -> HashSet<Uuid> {
    let mut reachable = HashSet::new();
    let mut queue = VecDeque::new();

    queue.push_back(start);
    reachable.insert(start);

    while let Some(node) = queue.pop_front() {
        if let Some(neighbors) = adj_list.get(&node) {
            for &neighbor in neighbors {
                if reachable.insert(neighbor) {
                    queue.push_back(neighbor);
                }
            }
        }
    }

    reachable
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_would_create_cycle_simple() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();

        let mut adj_list = HashMap::new();
        adj_list.insert(a, vec![b]);
        adj_list.insert(b, vec![c]);

        // c -> a would create cycle: a -> b -> c -> a
        assert!(would_create_cycle(&adj_list, c, a));

        // c -> b would also create cycle: b -> c -> b
        assert!(would_create_cycle(&adj_list, c, b));
    }

    #[test]
    fn test_would_create_cycle_direct() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();

        let mut adj_list = HashMap::new();
        adj_list.insert(a, vec![b]);

        assert!(would_create_cycle(&adj_list, b, a));
    }

    #[test]
    fn test_would_create_cycle_no_path() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();
        let d = Uuid::new_v4();

        let mut adj_list = HashMap::new();
        adj_list.insert(a, vec![b]);
        adj_list.insert(c, vec![d]);

        assert!(!would_create_cycle(&adj_list, d, a));
    }

    #[test]
    fn test_has_cycle_simple() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();

        let mut adj_list = HashMap::new();
        adj_list.insert(a, vec![b]);
        adj_list.insert(b, vec![c]);
        adj_list.insert(c, vec![a]);

        assert!(has_cycle(&adj_list));
    }

    #[test]
    fn test_has_cycle_no_cycle() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();

        let mut adj_list = HashMap::new();
        adj_list.insert(a, vec![b]);
        adj_list.insert(b, vec![c]);

        assert!(!has_cycle(&adj_list));
    }

    #[test]
    fn test_reachable_from() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();
        let d = Uuid::new_v4();

        let mut adj_list = HashMap::new();
        adj_list.insert(a, vec![b, c]);
        adj_list.insert(b, vec![d]);

        let reachable = reachable_from(&adj_list, a);
        assert_eq!(reachable.len(), 4);
        assert!(reachable.contains(&a));
        assert!(reachable.contains(&b));
        assert!(reachable.contains(&c));
        assert!(reachable.contains(&d));
    }

    #[test]
    fn test_reachable_from_isolated() {
        let a = Uuid::new_v4();
        let _b = Uuid::new_v4();

        let adj_list = HashMap::new();

        let reachable = reachable_from(&adj_list, a);
        assert_eq!(reachable.len(), 1);
        assert!(reachable.contains(&a));
    }
}
