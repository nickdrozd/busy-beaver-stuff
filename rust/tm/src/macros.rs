use core::{cell::RefCell, iter::once};

use std::collections::BTreeMap as Dict;

use num_integer::Integer as _;

use crate::{
    instrs::{Color, Instr, Params, Shift, Slot, State},
    prog::Prog,
};

type MacroColor = u64;
type MacroState = u64;

type MacroSlot = (MacroState, MacroColor);
type MacroInstr = (MacroColor, Shift, MacroState);

type MacroInstrs = Dict<MacroSlot, MacroInstr>;

type Tape = Vec<Color>;
type Config = (State, (bool, Tape));

/**************************************/

pub trait GetInstr {
    fn get_instr(&self, slot: Slot) -> Option<Instr>;
}

impl GetInstr for Prog {
    fn get_instr(&self, slot: Slot) -> Option<Instr> {
        self.get(slot).copied()
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

    fn macro_states(&self) -> MacroState {
        MacroState::from(2 * self.base_states())
    }

    fn macro_colors(&self) -> MacroColor {
        let base = MacroColor::from(self.base_colors());
        let exp = u32::try_from(self.cells()).unwrap();

        base.pow(exp)
    }

    fn sim_lim(&self) -> usize {
        self.cells()
            * self.base_states() as usize
            * usize::try_from(self.macro_colors()).unwrap()
    }

    fn deconstruct_inputs(
        &self,
        (macro_state, macro_color): MacroSlot,
    ) -> Config {
        let (state, right_edge) = macro_state.div_rem(&2_u8.into());

        (
            State::try_from(state).unwrap(),
            (
                right_edge == 1,
                self.converter.color_to_tape(&macro_color),
            ),
        )
    }

    fn reconstruct_outputs(
        &self,
        (state, (right_edge, tape)): Config,
    ) -> MacroInstr {
        (
            self.converter.tape_to_color(tape),
            right_edge,
            MacroState::from(2 * state) + MacroState::from(!right_edge),
        )
    }
}

/**************************************/

#[expect(private_bounds)]
pub struct MacroProg<'p, L: Logic> {
    prog: &'p Prog,
    logic: L,

    instrs: RefCell<MacroInstrs>,

    states: RefCell<Vec<MacroState>>,
    colors: RefCell<Vec<MacroColor>>,
}

impl<L: Logic> GetInstr for MacroProg<'_, L> {
    fn get_instr(&self, slot: Slot) -> Option<Instr> {
        let slot: MacroSlot = self.convert_slot(slot);

        let (color, shift, state) = {
            if let Some(&instr) = self.instrs.borrow().get(&slot) {
                instr
            } else {
                let instr = self.calculate_instr(slot)?;

                self.cache_instr(slot, instr);

                instr
            }
        };

        Some((
            self.convert_color(color),
            shift,
            self.convert_state(state),
        ))
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

    fn convert_slot(&self, (state, color): Slot) -> MacroSlot {
        (
            MacroState::from(self.states.borrow()[state as usize]),
            MacroColor::from(self.colors.borrow()[color as usize]),
        )
    }

    #[expect(clippy::cast_possible_truncation)]
    fn convert_state(&self, state: MacroState) -> State {
        let mut states = self.states.borrow_mut();

        let pos = states
            .iter()
            .position(|s| *s == state)
            .unwrap_or_else(|| {
                states.push(state);
                states.len() - 1
            });

        pos as State
    }

    #[expect(clippy::cast_possible_truncation)]
    fn convert_color(&self, color: MacroColor) -> Color {
        let mut colors = self.colors.borrow_mut();

        let pos = colors
            .iter()
            .position(|s| *s == color)
            .unwrap_or_else(|| {
                colors.push(color);
                colors.len() - 1
            });

        pos as Color
    }

    fn cache_instr(&self, slot: MacroSlot, instr: MacroInstr) {
        self.instrs.borrow_mut().insert(slot, instr);
    }

    fn calculate_instr(&self, slot: MacroSlot) -> Option<MacroInstr> {
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
                self.prog.get((state, scan))?;

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

            backsymbols: {
                let base = base_colors as usize;
                let exp = u32::try_from(cells).unwrap();

                base.pow(exp)
            },

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

    fn macro_states(&self) -> MacroState {
        MacroState::from(
            2 * self.base_states
                * State::try_from(self.backsymbols).unwrap(),
        )
    }

    fn macro_colors(&self) -> MacroColor {
        MacroColor::from(self.base_colors)
    }

    fn sim_lim(&self) -> usize {
        usize::try_from(self.macro_states()).unwrap()
            * usize::try_from(self.macro_colors()).unwrap()
    }

    fn deconstruct_inputs(
        &self,
        (macro_state, macro_color): MacroSlot,
    ) -> Config {
        let macro_color = Color::try_from(macro_color).unwrap();

        let (st_co, at_right) = macro_state.div_rem(&2_u8.into());

        let st_co = State::try_from(st_co).unwrap();

        let (state, color) = st_co
            .div_rem(&(Color::try_from(self.backsymbols).unwrap()));

        let backspan =
            self.converter.color_to_tape(&MacroColor::from(color));

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
    ) -> MacroInstr {
        let shift = !right_edge;

        let (backspan, macro_color) = if shift {
            let (rest, color) = tape.split_at(self.cells - 1);
            (rest.to_vec(), color[0])
        } else {
            let (color, rest) = tape.split_at(1);
            (rest.to_vec(), color[0])
        };

        (
            MacroColor::from(macro_color),
            shift,
            MacroColor::from(shift)
                + (MacroColor::from(2_u8)
                    * (MacroColor::from(
                        state
                            * (Color::try_from(self.backsymbols)
                                .unwrap()),
                    ) + self.converter.tape_to_color(backspan))),
        )
    }
}

/**************************************/

trait Logic {
    fn new(cells: usize, params: Params) -> Self;

    fn cells(&self) -> usize;

    fn base_states(&self) -> State;
    fn base_colors(&self) -> Color;

    fn macro_states(&self) -> MacroState;
    fn macro_colors(&self) -> MacroColor;

    fn sim_lim(&self) -> usize;

    fn deconstruct_inputs(&self, slot: MacroSlot) -> Config;
    fn reconstruct_outputs(&self, config: Config) -> MacroInstr;
}

/**************************************/

struct TapeColorConverter {
    base_colors: Color,

    ct_cache: RefCell<Dict<MacroColor, Tape>>,
    tc_cache: RefCell<Dict<Tape, MacroColor>>,
}

impl TapeColorConverter {
    fn new(base_colors: Color, cells: usize) -> Self {
        let mut ct_cache = Dict::new();

        ct_cache.insert(0, vec![0; cells]);

        Self {
            base_colors,
            ct_cache: RefCell::new(ct_cache),
            tc_cache: Dict::new().into(),
        }
    }

    #[expect(clippy::trivially_copy_pass_by_ref)]
    fn color_to_tape(&self, color: &MacroColor) -> Tape {
        self.ct_cache.borrow()[color].clone()
    }

    fn tape_to_color(&self, tape: Tape) -> MacroColor {
        if let Some(&color) = self.tc_cache.borrow().get(&tape) {
            return color;
        }

        let color = tape
            .iter()
            .map(|c| MacroColor::from(*c))
            .rev()
            .enumerate()
            .fold(MacroColor::MIN, |acc, (place, value)| {
                acc + value * {
                    let base = MacroColor::from(self.base_colors);
                    let exp = u32::try_from(place).unwrap();

                    base.pow(exp)
                }
            });

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
    fn params(&self) -> (MacroState, MacroColor) {
        (self.logic.macro_states(), self.logic.macro_colors())
    }

    pub fn assert_params(&self, (states, colors): (u16, u16)) {
        let (mac_states, mac_colors) = self.params();

        assert_eq!(mac_states, MacroState::from(states));
        assert_eq!(mac_colors, MacroColor::from(colors));
    }

    pub fn rep_params(&self) -> (usize, usize) {
        (self.states.borrow().len(), self.colors.borrow().len())
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
        assert_eq!(Some(instr), block.get_instr(slot));
    }
}
