#![expect(dead_code)]

use core::{cell::RefCell, iter::once};

use std::collections::{BTreeMap as Dict, BTreeSet as Set};

use crate::instrs::{
    Color, CompProg, GetInstr, Instr, Params, Shift, Slot, State,
};

type Tape = Vec<Color>;
type Config = (State, (bool, Tape));

/**************************************/

pub fn make_block_macro<P: GetInstr>(
    prog: &P,
    params: Params,
    blocks: usize,
) -> MacroProg<P, BlockLogic> {
    MacroProg::new(prog, BlockLogic::new(blocks, params))
}

pub fn make_backsymbol_macro<P: GetInstr>(
    prog: &P,
    params: Params,
    backsymbols: usize,
) -> MacroProg<P, BacksymbolLogic> {
    MacroProg::new(prog, BacksymbolLogic::new(backsymbols, params))
}

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

    fn name() -> &'static str {
        "block"
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
        let state = macro_state / 2;
        let right_edge = macro_state % 2;

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

    instrs: RefCell<CompProg>,
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

    fn halt_slots(&self) -> Set<Slot> {
        self.instrs.borrow().halt_slots()
    }

    fn erase_slots(&self) -> Set<Slot> {
        self.instrs.borrow().erase_slots()
    }

    fn zr_shifts(&self) -> Set<(State, Shift)> {
        self.instrs.borrow().zr_shifts()
    }

    #[cfg(test)]
    fn incomplete(&self, _params: Params, _halt: bool) -> bool {
        unimplemented!()
    }
}

#[expect(private_bounds)]
impl<'p, P: GetInstr, L: Logic> MacroProg<'p, P, L> {
    const fn new(prog: &'p P, logic: L) -> Self {
        Self {
            prog,
            logic,
            instrs: RefCell::new(Dict::new()),
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

    fn name() -> &'static str {
        "backsymbol"
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
        let (st_co, at_right) = (macro_state / 2, macro_state % 2);

        let state = st_co / self.backsymbols as Color;

        let backspan = self
            .converter
            .color_to_tape(st_co % self.backsymbols as Color);

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

trait Logic: Sized {
    fn new(cells: usize, params: Params) -> Self;

    fn name() -> &'static str;

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

    color_to_tape_cache: RefCell<Dict<Color, Tape>>,
    tape_to_color_cache: RefCell<Dict<Tape, Color>>,
}

impl TapeColorConverter {
    fn new(base_colors: Color, cells: usize) -> Self {
        let mut color_to_tape_cache = Dict::new();

        color_to_tape_cache.insert(0, vec![0; cells]);

        Self {
            base_colors,
            color_to_tape_cache: RefCell::new(color_to_tape_cache),
            tape_to_color_cache: RefCell::new(Dict::new()),
        }
    }

    fn color_to_tape(&self, color: Color) -> Tape {
        self.color_to_tape_cache.borrow()[&color].clone()
    }

    fn tape_to_color(&self, tape: Tape) -> Color {
        let mut tape_to_color_cache =
            self.tape_to_color_cache.borrow_mut();

        if let Some(&color) = tape_to_color_cache.get(&tape) {
            color
        } else {
            let color = tape.iter().rev().enumerate().fold(
                0,
                |acc, (place, &value)| {
                    acc + value * self.base_colors.pow(place as u32)
                },
            );

            tape_to_color_cache.insert(tape.clone(), color);

            self.color_to_tape_cache.borrow_mut().insert(color, tape);

            color
        }
    }
}

/**************************************/

#[cfg(test)]
use crate::instrs::Parse as _;

#[test]
fn test_macro() {
    let comp = CompProg::from_str(
        "1RB 1LC  1RC 1RB  1RD 0LE  1LA 1LD  ... 0LA",
    );

    let block = make_block_macro(&comp, (5, 2), 3);

    let _ = make_backsymbol_macro(&comp, (5, 2), 3);

    let _ = make_backsymbol_macro(&block, (5, 2), 3);
}
