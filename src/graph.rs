use std::collections::{HashMap, HashSet};

use pyo3::prelude::*;

use crate::instrs::{Color, State};
use crate::parse::{parse, st_str};

type ConGraph = HashMap<State, HashSet<State>>;

#[pyclass]
pub struct Graph {
    pub arrows: HashMap<State, Vec<Option<State>>>,
}

#[pymethods]
impl Graph {
    #[new]
    fn new(program: &str) -> Self {
        let arrows = parse(program)
            .into_iter()
            .enumerate()
            .map(|(state, instrs)| {
                (
                    state as State,
                    instrs.into_iter().map(|instr| instr.map(|x| x.2)).collect(),
                )
            })
            .collect();

        Self { arrows }
    }

    #[getter]
    fn arrows(&self) -> HashMap<State, Vec<Option<State>>> {
        self.arrows.clone()
    }

    fn __str__(&self) -> String {
        let mut sorted_arrows: Vec<(&State, &Vec<Option<State>>)> = self.arrows.iter().collect();
        sorted_arrows.sort_by_key(|(key, _)| *key);

        let result: String = sorted_arrows
            .iter()
            .flat_map(|(_, conn)| conn.iter())
            .map(|&dst| st_str(dst))
            .collect();

        result
    }

    #[getter]
    fn states(&self) -> Vec<State> {
        let mut states: Vec<_> = self.arrows.keys().copied().collect();
        states.sort_unstable();
        states
    }

    #[getter]
    fn colors(&self) -> Vec<Color> {
        (0..self.arrows[&0].len() as Color).collect()
    }

    #[getter]
    fn exit_points(&self) -> ConGraph {
        self.arrows
            .iter()
            .map(|(state, connections)| {
                (
                    *state,
                    connections
                        .iter()
                        .filter_map(|conn| {
                            if let Some(c) = conn {
                                if *c != -1 {
                                    return Some(*c);
                                }
                            }
                            None
                        })
                        .collect(),
                )
            })
            .collect()
    }

    #[getter]
    fn entry_points(&self) -> ConGraph {
        let mut entries: ConGraph = self
            .states()
            .iter()
            .map(|state| (*state, HashSet::new()))
            .collect();

        for (state, exits) in &self.exit_points() {
            for exit_point in exits {
                entries.get_mut(exit_point).unwrap().insert(*state);
            }
        }

        entries
    }

    #[getter]
    fn is_normal(&self) -> bool {
        let flat_graph = self.__str__();
        let states = self.states();

        if states
            .iter()
            .skip(1)
            .any(|&state| !flat_graph.contains(st_str(Some(state))))
        {
            return false;
        }

        let positions: Vec<usize> = states
            .iter()
            .skip(1)
            .map(|&state| flat_graph.find(st_str(Some(state))).unwrap())
            .collect();

        let mut sorted_positions = positions.clone();
        sorted_positions.sort_unstable();

        positions == sorted_positions
    }

    #[getter]
    fn is_strongly_connected(&self) -> bool {
        for state in self.states() {
            let mut reachable_from_x: HashSet<i32> =
                self.arrows[&state].iter().copied().flatten().collect();

            for _ in self.states() {
                let new_reachable: HashSet<i32> = reachable_from_x
                    .iter()
                    .filter_map(|&connection| self.arrows.get(&connection))
                    .flat_map(|nodes| nodes.iter())
                    .copied()
                    .flatten()
                    .collect();

                reachable_from_x.extend(new_reachable);
            }

            if reachable_from_x.len() < self.states().len() {
                return false;
            }
        }

        true
    }

    #[getter]
    fn reflexive_states(&self) -> HashSet<State> {
        self.arrows
            .iter()
            .filter(|(state, connections)| connections.contains(&Some(**state)))
            .map(|(&state, _)| state)
            .collect()
    }

    #[getter]
    fn is_irreflexive(&self) -> bool {
        self.reflexive_states().is_empty()
    }

    #[getter]
    fn zero_reflexive_states(&self) -> HashSet<State> {
        self.arrows
            .iter()
            .filter(|(&state, connections)| connections[0] == Some(state))
            .map(|(&state, _)| state)
            .collect()
    }

    #[getter]
    fn is_zero_reflexive(&self) -> bool {
        !self.zero_reflexive_states().is_empty()
    }

    #[getter]
    pub fn entries_dispersed(&self) -> bool {
        let colors_count = self.colors().len();
        self.entry_points()
            .values()
            .all(|entries| entries.len() == colors_count)
    }

    #[getter]
    pub fn exits_dispersed(&self) -> bool {
        let colors_count = self.colors().len();
        self.exit_points()
            .values()
            .all(|exits| exits.len() == colors_count)
    }

    #[getter]
    pub fn is_dispersed(&self) -> bool {
        self.entries_dispersed() && self.exits_dispersed()
    }

    #[getter]
    pub fn is_simple(&self) -> bool {
        self.reduced().is_empty()
    }

    #[getter]
    pub fn reduced(&self) -> ConGraph {
        reduce_graph(
            self.exit_points(),
            self.states().len() * self.colors().len(),
        )
    }
}

fn reduce_graph(mut graph: ConGraph, passes: usize) -> ConGraph {
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
    fn uwe_graph() {
        let mut graph: ConGraph = HashMap::new();

        graph.insert(1, vec![2, 3].into_iter().collect());
        graph.insert(2, vec![3, 4].into_iter().collect());
        graph.insert(3, vec![1, 2].into_iter().collect());
        graph.insert(4, vec![5].into_iter().collect());
        graph.insert(5, vec![1, 3].into_iter().collect());

        let reduced = reduce_graph(graph, 10);

        assert_eq!(reduced.len(), 3);
    }

    #[test]
    fn uwe_prog() {
        let graph = Graph::new("1RB 0LC  1RC 1RD  1LA 0RB  0RE 1R_  1LC 1RA");

        assert_eq!(graph.reduced().len(), 3);
    }

    #[test]
    fn test_normal() {
        assert!(Graph::new("1RB 1LB  1LA 0LC  1R_ 1LD  1RD 0RA").is_normal());
        assert!(!Graph::new("1RC 1LB  1LA 0LC  1R_ 1LD  1RC 0RA").is_normal());
    }
}
