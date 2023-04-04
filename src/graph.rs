use std::collections::{HashMap, HashSet};

use pyo3::prelude::*;

type State = u32;

type ConGraph = HashMap<State, HashSet<State>>;

#[pyfunction]
pub fn reduce_graph(mut graph: ConGraph, passes: usize) -> ConGraph {
    for _ in 0..passes {
        if graph.is_empty() {
            break;
        }

        cut_reflexive_arrows(&mut graph);
        inline_single_exit(&mut graph);
        inline_single_entry(&mut graph);
    }

    graph.retain(|_, connections| !connections.is_empty());
    graph
}

fn purge_dead_ends(graph: &mut ConGraph) {
    let to_cut: HashSet<_> = graph
        .iter()
        .filter(|(_, connections)| connections.is_empty())
        .map(|(state, _)| *state)
        .collect();

    for state in to_cut {
        for connections in graph.values_mut() {
            connections.remove(&state);
        }

        graph.remove(&state);
    }
}

fn inline_single_entry(graph: &mut ConGraph) {
    let mut changed = true;
    while changed {
        changed = false;

        let mut to_remove = None;
        let mut entry_point = None;
        let mut dst_connections = HashSet::new();

        for (dst, _) in graph.iter() {
            let entries: Vec<_> = graph
                .iter()
                .filter_map(|(src, connections)| {
                    if connections.contains(dst) {
                        Some(*src)
                    } else {
                        None
                    }
                })
                .collect();

            if entries.len() == 1 {
                entry_point = Some(entries[0]);
                dst_connections = graph.get(dst).unwrap().clone();
                to_remove = Some(*dst);
                changed = true;
                break;
            }
        }

        if let (Some(entry), Some(dst)) = (entry_point, to_remove) {
            let entry_connections = graph.get_mut(&entry).unwrap();
            entry_connections.extend(dst_connections);
            entry_connections.remove(&dst);
            graph.remove(&dst);
        }
    }

    purge_dead_ends(graph);
}

fn cut_reflexive_arrows(graph: &mut ConGraph) {
    for (state, connections) in graph.iter_mut() {
        connections.remove(state);
    }
}

fn inline_single_exit(graph: &mut ConGraph) {
    let mut changed = true;
    while changed {
        changed = false;

        let mut to_remove = None;
        let mut exit_point = None;

        for (state, connections) in graph.iter() {
            if connections.len() == 1 {
                let exit = connections.iter().next().unwrap();
                if *exit != *state {
                    exit_point = Some(*exit);
                    to_remove = Some(*state);
                    changed = true;
                    break;
                }
            }
        }

        if let (Some(state), Some(exit)) = (to_remove, exit_point) {
            for con_rep in graph.values_mut() {
                if con_rep.remove(&state) {
                    con_rep.insert(exit);
                }
            }
            graph.remove(&state);
        }
    }

    purge_dead_ends(graph);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uwe() {
        let mut graph: ConGraph = HashMap::new();

        graph.insert(1, vec![2, 3].into_iter().collect());
        graph.insert(2, vec![3, 4].into_iter().collect());
        graph.insert(3, vec![1, 2].into_iter().collect());
        graph.insert(4, vec![5].into_iter().collect());
        graph.insert(5, vec![1, 3].into_iter().collect());

        let reduced = reduce_graph(graph, 10);

        assert_eq!(reduced.len(), 3);
    }
}
