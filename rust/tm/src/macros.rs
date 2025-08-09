#![deny(clippy::cast_possible_truncation)]

use core::{cell::RefCell, iter::once};

use std::collections::BTreeMap as Dict;

use num_integer::Integer as _;

use crate::{
    instrs::{Color, Instr, Instrs, Params, Slot, State},
    prog::Prog,
};

type Tape = Vec<Color>;
type Config = (State, (bool, Tape));

/**************************************/

pub trait GetInstr {
    fn get_instr(&self, slot: &Slot) -> Option<Instr>;
}

impl GetInstr for Prog {
    fn get_instr(&self, slot: &Slot) -> Option<Instr> {
        self.instrs.get(slot).copied()
    }
}

/**************************************/

type BlockMacro<'p> = MacroProg<'p, BlockLogic>;
type BacksymbolMacro<'p> = MacroProg<'p, BacksymbolLogic>;

impl Prog {
    pub fn make_block_macro(&self, blocks: usize) -> BlockMacro<'_> {
        MacroProg::new(self, BlockLogic::new(blocks, self.params()))
    }

    pub fn make_backsymbol_macro(
        &self,
        backsymbols: usize,
    ) -> BacksymbolMacro<'_> {
        MacroProg::new(
            self,
            BacksymbolLogic::new(backsymbols, self.params()),
        )
    }
}

/**************************************/

pub struct BlockLogic {
    cells: usize,

    base_colors: Color,
    base_states: State,

    converter: TapeColorConverter,
}

impl Logic for BlockLogic {
    fn new(cells: usize, (base_states, base_colors): Params) -> Self {
        Self {
            cells,
            base_states,
            base_colors,

            converter: TapeColorConverter::new(base_colors, cells),
        }
    }

    fn cells(&self) -> usize {
        self.cells
    }

    fn base_states(&self) -> State {
        self.base_states
    }

    fn base_colors(&self) -> Color {
        self.base_colors
    }

    fn macro_states(&self) -> State {
        2 * self.base_states()
    }

    fn macro_colors(&self) -> Color {
        self.base_colors().pow(u32::try_from(self.cells()).unwrap())
    }

    fn sim_lim(&self) -> usize {
        self.cells()
            * usize::try_from(self.base_states()).unwrap()
            * usize::try_from(self.macro_colors()).unwrap()
    }

    fn deconstruct_inputs(
        &self,
        (macro_state, macro_color): Slot,
    ) -> Config {
        let (state, right_edge) = macro_state.div_rem(&2);

        (
            state,
            (
                right_edge == 1,
                self.converter.color_to_tape(macro_color),
            ),
        )
    }

    fn reconstruct_outputs(
        &self,
        (state, (right_edge, tape)): Config,
    ) -> Instr {
        (
            self.converter.tape_to_color(tape),
            right_edge,
            (2 * state) + State::from(!right_edge),
        )
    }
}

/**************************************/

#[expect(private_bounds)]
pub struct MacroProg<'p, L: Logic> {
    prog: &'p Prog,
    logic: L,

    instrs: RefCell<Instrs>,

    states: RefCell<Vec<State>>,
    colors: RefCell<Vec<Color>>,
}

impl<L: Logic> GetInstr for MacroProg<'_, L> {
    #[expect(clippy::cast_possible_truncation)]
    fn get_instr(&self, &(in_state, in_color): &Slot) -> Option<Instr> {
        let slot = (
            self.states.borrow()[in_state as usize],
            self.colors.borrow()[in_color as usize],
        );

        let (out_color, shift, out_state) = {
            if let Some(&instr) = self.instrs.borrow().get(&slot) {
                instr
            } else {
                let instr = self.calculate_instr(slot)?;

                self.instrs.borrow_mut().insert(slot, instr);

                instr
            }
        };

        let fwd_state = {
            let mut states = self.states.borrow_mut();

            states.iter().position(|&s| s == out_state).unwrap_or_else(
                || {
                    states.push(out_state);
                    states.len() - 1
                },
            ) as State
        };

        let fwd_color = {
            let mut colors = self.colors.borrow_mut();

            colors.iter().position(|&s| s == out_color).unwrap_or_else(
                || {
                    colors.push(out_color);
                    colors.len() - 1
                },
            ) as Color
        };

        Some((fwd_color, shift, fwd_state))
    }
}

#[expect(private_bounds)]
impl<'p, L: Logic> MacroProg<'p, L> {
    fn new(prog: &'p Prog, logic: L) -> Self {
        Self {
            prog,
            logic,
            instrs: Dict::new().into(),
            states: vec![0].into(),
            colors: vec![0].into(),
        }
    }

    fn calculate_instr(&self, slot: Slot) -> Option<Instr> {
        Some(self.logic.reconstruct_outputs(
            self.run_simulator(self.logic.deconstruct_inputs(slot))?,
        ))
    }

    fn run_simulator(
        &self,
        (mut state, (right_edge, mut tape)): Config,
    ) -> Option<Config> {
        let cells = tape.len();

        let mut pos = if right_edge { cells - 1 } else { 0 };

        let mut side: Option<bool> = None;

        'step: for _ in 0..self.logic.sim_lim() {
            let scan = tape[pos];

            let &(color, shift, next_state) =
                self.prog.get(&(state, scan))?;

            if next_state != state {
                state = next_state;

                tape[pos] = color;

                if shift {
                    pos += 1;
                    if cells <= pos {
                        side = Some(true);
                        break;
                    }
                } else {
                    if pos == 0 {
                        side = Some(false);
                        break;
                    }
                    pos -= 1;
                }
            } else if shift {
                while tape[pos] == scan {
                    tape[pos] = color;
                    pos += 1;
                    if cells <= pos {
                        side = Some(true);
                        break 'step;
                    }
                }
            } else {
                while tape[pos] == scan {
                    tape[pos] = color;
                    if pos == 0 {
                        side = Some(false);
                        break 'step;
                    }
                    pos -= 1;
                }
            }
        }

        side.map(|side| (state, (side, tape)))
    }
}

/**************************************/

pub struct BacksymbolLogic {
    cells: usize,
    base_states: State,
    base_colors: Color,
    backsymbols: usize,

    converter: TapeColorConverter,
}

impl Logic for BacksymbolLogic {
    fn new(cells: usize, (base_states, base_colors): Params) -> Self {
        Self {
            cells,
            base_states,
            base_colors,

            backsymbols: (usize::try_from(base_colors).unwrap())
                .pow(u32::try_from(cells).unwrap()),

            converter: TapeColorConverter::new(base_colors, cells),
        }
    }

    fn cells(&self) -> usize {
        self.cells
    }

    fn base_states(&self) -> State {
        self.base_states
    }

    fn base_colors(&self) -> Color {
        self.base_colors
    }

    fn macro_states(&self) -> State {
        2 * self.base_states * self.backsymbols as State
    }

    fn macro_colors(&self) -> Color {
        self.base_colors
    }

    fn sim_lim(&self) -> usize {
        usize::try_from(self.macro_states()).unwrap()
            * usize::try_from(self.macro_colors()).unwrap()
    }

    fn deconstruct_inputs(
        &self,
        (macro_state, macro_color): Slot,
    ) -> Config {
        let (st_co, at_right) = macro_state.div_rem(&2);

        let (state, color) =
            st_co.div_rem(&(self.backsymbols as Color));

        let backspan = self.converter.color_to_tape(color);

        (
            state,
            if at_right == 1 {
                (false, once(macro_color).chain(backspan).collect())
            } else {
                (
                    true,
                    backspan
                        .into_iter()
                        .chain(once(macro_color))
                        .collect(),
                )
            },
        )
    }

    fn reconstruct_outputs(
        &self,
        (state, (right_edge, tape)): Config,
    ) -> Instr {
        let shift = !right_edge;

        let (backspan, macro_color) = if shift {
            let (rest, color) = tape.split_at(self.cells - 1);
            (rest.to_vec(), color[0])
        } else {
            let (color, rest) = tape.split_at(1);
            (rest.to_vec(), color[0])
        };

        (
            macro_color,
            shift,
            Color::from(shift)
                + (2 * ((state * (self.backsymbols as Color))
                    + self.converter.tape_to_color(backspan))),
        )
    }
}

/**************************************/

trait Logic {
    fn new(cells: usize, params: Params) -> Self;

    fn cells(&self) -> usize;

    fn base_states(&self) -> State;
    fn base_colors(&self) -> Color;

    fn macro_states(&self) -> State;
    fn macro_colors(&self) -> Color;

    fn sim_lim(&self) -> usize;

    fn deconstruct_inputs(&self, slot: Slot) -> Config;
    fn reconstruct_outputs(&self, config: Config) -> Instr;
}

/**************************************/

struct TapeColorConverter {
    base_colors: Color,

    ct_cache: RefCell<Dict<Color, Tape>>,
    tc_cache: RefCell<Dict<Tape, Color>>,
}

impl TapeColorConverter {
    fn new(base_colors: Color, cells: usize) -> Self {
        let mut ct_cache = Dict::new();

        ct_cache.insert(0, vec![0; cells]);

        Self {
            base_colors,
            ct_cache: ct_cache.into(),
            tc_cache: Dict::new().into(),
        }
    }

    fn color_to_tape(&self, color: Color) -> Tape {
        self.ct_cache.borrow()[&color].clone()
    }

    fn tape_to_color(&self, tape: Tape) -> Color {
        if let Some(&color) = self.tc_cache.borrow().get(&tape) {
            return color;
        }

        let color = tape.iter().rev().enumerate().fold(
            0,
            |acc, (place, &value)| {
                acc + value
                    * self
                        .base_colors
                        .pow(u32::try_from(place).unwrap())
            },
        );

        self.tc_cache.borrow_mut().insert(tape.clone(), color);

        self.ct_cache.borrow_mut().insert(color, tape);

        color
    }
}

/**************************************/

#[cfg(test)]
use crate::instrs::Parse as _;

#[cfg(test)]
#[expect(private_bounds)]
impl<L: Logic> MacroProg<'_, L> {
    fn params(&self) -> Params {
        (self.logic.macro_states(), self.logic.macro_colors())
    }

    pub fn assert_params(&self, (states, colors): (State, Color)) {
        let (mac_states, mac_colors) = self.params();

        assert_eq!(mac_states, states);
        assert_eq!(mac_colors, colors);
    }
}

#[test]
fn test_params() {
    let prog =
        Prog::read("1RB 1LC  1RC 1RB  1RD 0LE  1LA 1LD  ... 0LA");

    let block = prog.make_block_macro(3);

    block.assert_params((10, 8));

    let backs = prog.make_backsymbol_macro(3);

    backs.assert_params((80, 2));
}

#[cfg(test)]
const MACROS: &[(Slot, Instr)] = &[
    ((0, 0), (1, true, 1)),
    ((1, 0), (2, false, 2)),
    ((2, 1), (2, false, 3)),
    ((3, 0), (1, true, 4)),
    ((4, 2), (2, false, 2)),
];

#[test]
fn test_macro() {
    let prog =
        Prog::read("0RB 0LC  1LA 1RB  1RD 0RE  1LC 1LA  ... 0LD");

    let block = prog.make_block_macro(2);

    block.assert_params((10, 4));

    for &(slot, instr) in MACROS {
        assert_eq!(Some(instr), block.get_instr(&slot));
    }
}
