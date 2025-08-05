use core::{cell::RefCell, iter::once};

use std::collections::BTreeMap as Dict;

use num_integer::Integer as _;

use crate::instrs::{
    Color, GetInstr, Instr, Instrs, Params, Slot, State,
};

type Tape = Vec<Color>;
type Config = (State, (bool, Tape));

/**************************************/

type BlockMacro<'p, P> = MacroProg<'p, P, BlockLogic>;
type BacksymbolMacro<'p, P> = MacroProg<'p, P, BacksymbolLogic>;

pub trait Macro: GetInstr + Sized {
    fn make_block_macro(&self, blocks: usize) -> BlockMacro<'_, Self> {
        MacroProg::new(self, BlockLogic::new(blocks, self.params()))
    }

    fn make_backsymbol_macro(
        &self,
        backsymbols: usize,
    ) -> BacksymbolMacro<'_, Self> {
        MacroProg::new(
            self,
            BacksymbolLogic::new(backsymbols, self.params()),
        )
    }
}

impl<T: GetInstr> Macro for T {}

/**************************************/

pub struct BlockLogic {
    cells: usize,

    base_colors: usize,
    base_states: usize,

    converter: TapeColorConverter,
}

impl Logic for BlockLogic {
    fn new(cells: usize, (base_states, base_colors): Params) -> Self {
        Self {
            cells,
            base_states: base_states as usize,
            base_colors: base_colors as usize,

            converter: TapeColorConverter::new(base_colors, cells),
        }
    }

    fn cells(&self) -> usize {
        self.cells
    }

    fn base_states(&self) -> usize {
        self.base_states
    }

    fn base_colors(&self) -> usize {
        self.base_colors
    }

    fn macro_states(&self) -> usize {
        2 * self.base_states()
    }

    fn macro_colors(&self) -> usize {
        self.base_colors().pow(self.cells() as u32)
    }

    fn sim_lim(&self) -> usize {
        self.base_states() * self.cells() * self.macro_colors()
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
pub struct MacroProg<'p, P: GetInstr, L: Logic> {
    prog: &'p P,
    logic: L,

    instrs: RefCell<Instrs>,
}

impl<P: GetInstr, L: Logic> GetInstr for MacroProg<'_, P, L> {
    fn get_instr(&self, slot: &Slot) -> Option<Instr> {
        if let Some(&instr) = self.instrs.borrow().get(slot) {
            return Some(instr);
        }

        let instr = self.calculate_instr(*slot)?;

        self.instrs.borrow_mut().insert(*slot, instr);

        Some(instr)
    }

    fn params(&self) -> Params {
        (
            self.logic.macro_states() as State,
            self.logic.macro_colors() as Color,
        )
    }
}

#[expect(private_bounds)]
impl<'p, P: GetInstr, L: Logic> MacroProg<'p, P, L> {
    fn new(prog: &'p P, logic: L) -> Self {
        Self {
            prog,
            logic,
            instrs: Dict::new().into(),
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

            let (color, shift, next_state) =
                self.prog.get_instr(&(state, scan))?;

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
    base_states: usize,
    base_colors: usize,
    backsymbols: usize,

    converter: TapeColorConverter,
}

impl Logic for BacksymbolLogic {
    fn new(cells: usize, (base_states, base_colors): Params) -> Self {
        Self {
            cells,
            base_states: base_states as usize,
            base_colors: base_colors as usize,

            backsymbols: (base_colors as usize).pow(cells as u32),

            converter: TapeColorConverter::new(base_colors, cells),
        }
    }

    fn cells(&self) -> usize {
        self.cells
    }

    fn base_states(&self) -> usize {
        self.base_states
    }

    fn base_colors(&self) -> usize {
        self.base_colors
    }

    fn macro_states(&self) -> usize {
        2 * self.base_states * self.backsymbols
    }

    fn macro_colors(&self) -> usize {
        self.base_colors
    }

    fn sim_lim(&self) -> usize {
        self.macro_states() * self.macro_colors()
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

    fn base_states(&self) -> usize;
    fn base_colors(&self) -> usize;

    fn macro_states(&self) -> usize;
    fn macro_colors(&self) -> usize;

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
                acc + value * self.base_colors.pow(place as u32)
            },
        );

        self.tc_cache.borrow_mut().insert(tape.clone(), color);

        self.ct_cache.borrow_mut().insert(color, tape);

        color
    }
}

/**************************************/

#[cfg(test)]
use crate::{instrs::Parse as _, prog::Prog};

#[test]
fn test_nest() {
    let prog =
        Prog::read("1RB 1LC  1RC 1RB  1RD 0LE  1LA 1LD  ... 0LA");

    let block = prog.make_block_macro(3);

    let _ = prog.make_backsymbol_macro(3);

    let _ = block.make_backsymbol_macro(3);
}

#[cfg(test)]
const MACROS: &[(Slot, Instr)] = &[
    ((0, 0), (1, true, 2)),
    ((2, 0), (2, false, 1)),
    ((1, 1), (2, false, 5)),
    ((5, 0), (1, true, 6)),
    ((6, 2), (2, false, 1)),
];

#[test]
fn test_macro() {
    let prog =
        Prog::read("0RB 0LC  1LA 1RB  1RD 0RE  1LC 1LA  ... 0LD");

    let block = prog.make_block_macro(2);

    for &(slot, instr) in MACROS {
        assert_eq!(Some(instr), block.get_instr(&slot));
    }
}
