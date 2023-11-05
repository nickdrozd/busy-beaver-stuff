use std::collections::{HashMap, HashSet};

use pyo3::prelude::*;

use crate::graph::Graph;
use crate::instrs::{Color, Instr, Slot, State};
use crate::parse::{parse, show_instr};

pub type Switch = HashMap<Color, Option<Instr>>;

#[pyclass(subclass)]
pub struct Program {
    #[pyo3(get, set)]
    prog: HashMap<State, Switch>,

    #[pyo3(get)]
    graph: Graph,
}

#[pymethods]
impl Program {
    #[new]
    pub fn new(program: &str) -> Self {
        let prog = parse(program)
            .into_iter()
            .enumerate()
            .map(|(state, instrs)| {
                (
                    state as State,
                    instrs
                        .into_iter()
                        .enumerate()
                        .map(|(color, instr)| (color as Color, instr))
                        .collect(),
                )
            })
            .collect();

        let graph = Graph::new(program);

        Self { prog, graph }
    }

    pub fn __str__(&self) -> String {
        let mut items: Vec<_> = self.prog.iter().collect();
        items.sort_by_key(|(state, _)| *state);

        items
            .into_iter()
            .map(|(_, instrs)| {
                let mut instr_items: Vec<_> = instrs.iter().collect();
                instr_items.sort_by_key(|(color, _)| *color);

                instr_items
                    .into_iter()
                    .map(|(_, instr)| show_instr(*instr))
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect::<Vec<_>>()
            .join("  ")
    }

    pub fn __getitem__(&self, slot: Slot) -> Option<Instr> {
        let (state, color) = slot;
        self.prog
            .get(&state)
            .and_then(|instrs| instrs.get(&color))
            .copied()
            .flatten()
    }

    pub fn __setitem__(&mut self, slot: Slot, instr: Option<Instr>) {
        let (state, color) = slot;

        if let Some(switch) = self.prog.get_mut(&state) {
            switch.insert(color, instr);
        }
    }

    #[getter]
    pub fn states(&self) -> HashSet<State> {
        self.prog.keys().copied().collect()
    }

    #[getter]
    pub fn colors(&self) -> HashSet<Color> {
        (0..self.prog.get(&0).unwrap().len())
            .map(|x| x as Color)
            .collect()
    }

    #[getter]
    pub fn state_switches(&self) -> Vec<(State, Switch)> {
        let mut switches: Vec<(State, Switch)> =
            self.prog.iter().map(|(k, v)| (*k, v.clone())).collect();
        switches.sort_by_key(|(state, _)| *state);
        switches
    }

    #[getter]
    pub fn instr_slots(&self) -> Vec<(Slot, Option<Instr>)> {
        let mut slots = vec![];
        for (state, instrs) in &self.prog {
            for (color, instr) in instrs {
                slots.push(((*state, *color), *instr));
            }
        }
        slots
    }

    #[getter]
    pub fn used_instr_slots(&self) -> Vec<(Slot, Instr)> {
        self.instr_slots()
            .into_iter()
            .filter_map(|(slot, instr)| instr.map(|instr| (slot, instr)))
            .collect()
    }

    #[getter]
    pub fn instructions(&self) -> Vec<Option<Instr>> {
        self.prog
            .values()
            .flat_map(|instrs| instrs.values().copied())
            .collect()
    }

    #[getter]
    pub fn used_instructions(&self) -> Vec<Instr> {
        self.instructions().into_iter().flatten().collect()
    }

    #[getter]
    pub fn slots(&self) -> Vec<Slot> {
        self.instr_slots()
            .into_iter()
            .map(|(slot, _)| slot)
            .collect()
    }

    #[getter]
    pub fn open_slots(&self) -> Vec<Slot> {
        self.instr_slots()
            .into_iter()
            .filter(|(_, instr)| instr.is_none())
            .map(|(slot, _)| slot)
            .collect()
    }

    #[getter]
    pub fn last_slot(&self) -> Option<Slot> {
        let slots = self.open_slots();

        if slots.len() != 1 {
            return None;
        }

        Some(slots[0])
    }

    #[getter]
    pub fn halt_slots(&self) -> Vec<Slot> {
        self.instr_slots()
            .into_iter()
            .filter(|(_, instr)| instr.is_none() || instr.as_ref().unwrap().2 == -1)
            .map(|(slot, _)| slot)
            .collect()
    }

    #[getter]
    pub fn erase_slots(&self) -> Vec<Slot> {
        self.used_instr_slots()
            .into_iter()
            .filter(|(slot, instr)| slot.1 != 0 && instr.0 == 0)
            .map(|(slot, _)| slot)
            .collect()
    }

    #[getter]
    pub fn spinout_slots(&self) -> Vec<Slot> {
        self.graph
            .zero_reflexive_states()
            .iter()
            .map(|&state| (state, 0))
            .collect()
    }

    #[getter]
    pub fn used_states(&self) -> HashSet<State> {
        self.used_instructions()
            .into_iter()
            .map(|(_, _, state)| state)
            .collect()
    }

    #[getter]
    pub fn available_states(&self) -> Vec<State> {
        let used: HashSet<State> = self
            .used_states()
            .iter()
            .copied()
            .chain(std::iter::once(0))
            .collect();
        let mut diff: Vec<_> = self.states().difference(&used).copied().collect();
        diff.sort_unstable();

        let mut available: Vec<_> = used.into_iter().collect();

        if !diff.is_empty() {
            available.push(diff[0]);
        }

        available.sort_unstable();
        available
    }

    #[getter]
    pub fn used_colors(&self) -> HashSet<Color> {
        self.used_instructions()
            .into_iter()
            .map(|(color, _, _)| color)
            .collect()
    }

    #[getter]
    pub fn available_colors(&self) -> Vec<Color> {
        let used: HashSet<Color> = self
            .used_colors()
            .iter()
            .copied()
            .chain(std::iter::once(0))
            .collect();
        let mut diff: Vec<_> = self.colors().difference(&used).copied().collect();
        diff.sort_unstable();

        let mut available: Vec<_> = used.into_iter().collect();

        if !diff.is_empty() {
            available.push(diff[0]);
        }

        available.sort_unstable();
        available
    }

    #[getter]
    pub fn available_instrs(&self) -> Vec<Instr> {
        let available_colors = self.available_colors();
        let available_states = self.available_states();
        let mut instrs: Vec<Instr> = Vec::new();

        for color in &available_colors {
            for &state in &available_states {
                instrs.push((*color, false, state));
                instrs.push((*color, true, state));
            }
        }

        instrs.sort_by(|a, b| b.partial_cmp(a).unwrap());
        instrs
    }

    pub fn branch(&mut self, slot: Slot) -> Vec<String> {
        let mut branches = Vec::new();
        let orig = self.__getitem__(slot);

        for instr in self.available_instrs() {
            if let Some(orig_instr) = orig {
                if instr >= orig_instr {
                    continue;
                }
            }

            self.__setitem__(slot, Some(instr));
            branches.push(self.__str__());
        }

        self.__setitem__(slot, orig);
        branches
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uwe() {
        let prog: BasicProgramRust =
            BasicProgramRust::new("1RB 0LC  1RC 1RD  1LA 0RB  0RE 1R_  1LC 1RA");

        assert!(prog.prog.len() == 5);

        for switch in prog.prog.values() {
            assert!(switch.len() == 2);
        }
    }
}
