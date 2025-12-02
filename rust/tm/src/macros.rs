use core::{cell::RefCell, iter::once};

use std::collections::BTreeMap as Dict;

use num_integer::Integer as _;

use crate::{Color, Instr, Prog, Shift, Slot, State};

type MacroColor = u64;
type MacroState = u64;

type MacroSlot = (MacroState, MacroColor);
type MacroInstr = (MacroColor, Shift, MacroState);

type MacroInstrs = Dict<MacroSlot, MacroInstr>;

type Tape = Vec<Color>;
type Config = (State, (bool, Tape));

/**************************************/

#[derive(Debug, PartialEq, Eq)]
pub enum MacroExc {
    InfLoop,
}

pub type GetInstrResult = Result<Option<Instr>, MacroExc>;

pub trait GetInstr {
    fn get_instr(&self, slot: &Slot) -> GetInstrResult;
}

impl<const s: usize, const c: usize> GetInstr for Prog<s, c> {
    fn get_instr(&self, slot: &Slot) -> GetInstrResult {
        Ok(self.get(slot).copied())
    }
}

/**************************************/

type BlockMacro<'p, const s: usize, const c: usize> =
    MacroProg<'p, s, c, BlockLogic<s, c>>;

type BacksymbolMacro<'p, const s: usize, const c: usize> =
    MacroProg<'p, s, c, BacksymbolLogic<s, c>>;

impl<const states: usize, const colors: usize> Prog<states, colors> {
    pub fn make_block_macro(
        &self,
        blocks: usize,
    ) -> BlockMacro<'_, states, colors> {
        MacroProg::new(self, BlockLogic::new(blocks))
    }

    pub fn make_backsymbol_macro(
        &self,
        backsymbols: usize,
    ) -> BacksymbolMacro<'_, states, colors> {
        MacroProg::new(self, BacksymbolLogic::new(backsymbols))
    }
}

/**************************************/

pub struct BlockLogic<
    const base_states: usize,
    const base_colors: usize,
> {
    cells: usize,

    converter: TapeColorConverter<base_colors>,
}

impl<const base_states: usize, const base_colors: usize>
    Logic<base_states, base_colors>
    for BlockLogic<base_states, base_colors>
{
    fn new(cells: usize) -> Self {
        Self {
            cells,
            converter: TapeColorConverter::new(cells),
        }
    }

    fn cells(&self) -> usize {
        self.cells
    }

    fn macro_states(&self) -> MacroState {
        2 * base_states as MacroState
    }

    fn macro_colors(&self) -> MacroColor {
        let base = base_colors as MacroColor;
        let exp = u32::try_from(self.cells()).unwrap();

        base.pow(exp)
    }

    fn sim_lim(&self) -> usize {
        self.cells()
            * base_states
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
pub struct MacroProg<'p, const s: usize, const c: usize, L: Logic<s, c>>
{
    prog: &'p Prog<s, c>,
    logic: L,

    instrs: RefCell<MacroInstrs>,

    states: RefCell<Vec<MacroState>>,
    colors: RefCell<Vec<MacroColor>>,
}

impl<const s: usize, const c: usize, L: Logic<s, c>> GetInstr
    for MacroProg<'_, s, c, L>
{
    fn get_instr(&self, slot: &Slot) -> GetInstrResult {
        let slot: MacroSlot = self.convert_slot(slot);

        let (color, shift, state) = {
            if let Some(&instr) = self.instrs.borrow().get(&slot) {
                instr
            } else {
                let Some(instr) = self.calculate_instr(slot)? else {
                    return Ok(None);
                };

                self.cache_instr(slot, instr);

                instr
            }
        };

        Ok(Some((
            self.convert_color(color),
            shift,
            self.convert_state(state),
        )))
    }
}

#[expect(private_bounds)]
impl<'p, const st: usize, const co: usize, L: Logic<st, co>>
    MacroProg<'p, st, co, L>
{
    fn new(prog: &'p Prog<st, co>, logic: L) -> Self {
        Self {
            prog,
            logic,
            instrs: Dict::new().into(),
            states: vec![0].into(),
            colors: vec![0].into(),
        }
    }

    fn convert_slot(&self, &(state, color): &Slot) -> MacroSlot {
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

    fn calculate_instr(
        &self,
        slot: MacroSlot,
    ) -> Result<Option<MacroInstr>, MacroExc> {
        self.run_simulator(self.logic.deconstruct_inputs(slot))?
            .map_or(Ok(None), |config| {
                Ok(Some(self.logic.reconstruct_outputs(config)))
            })
    }

    fn run_simulator(
        &self,
        (mut state, (right_edge, mut tape)): Config,
    ) -> Result<Option<Config>, MacroExc> {
        let cells = tape.len();

        let mut pos = if right_edge { cells - 1 } else { 0 };

        let mut side: Option<bool> = None;

        'step: for _ in 0..self.logic.sim_lim() {
            let scan = tape[pos];

            let Some(&(color, shift, next_state)) =
                self.prog.get(&(state, scan))
            else {
                return Ok(None);
            };

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

        side.map_or(Err(MacroExc::InfLoop), |side| {
            Ok(Some((state, (side, tape))))
        })
    }
}

/**************************************/

pub struct BacksymbolLogic<
    const base_states: usize,
    const base_colors: usize,
> {
    cells: usize,
    backsymbols: usize,

    converter: TapeColorConverter<base_colors>,
}

impl<const base_states: usize, const base_colors: usize>
    Logic<base_states, base_colors>
    for BacksymbolLogic<base_states, base_colors>
{
    fn new(cells: usize) -> Self {
        Self {
            cells,

            backsymbols: {
                let base = base_colors;
                let exp = u32::try_from(cells).unwrap();

                base.pow(exp)
            },

            converter: TapeColorConverter::new(cells),
        }
    }

    fn cells(&self) -> usize {
        self.cells
    }

    fn macro_states(&self) -> MacroState {
        (2 * base_states * self.backsymbols) as MacroState
    }

    fn macro_colors(&self) -> MacroColor {
        base_colors as MacroColor
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

trait Logic<const states: usize, const colors: usize> {
    fn new(cells: usize) -> Self;

    fn cells(&self) -> usize;

    fn macro_states(&self) -> MacroState;
    fn macro_colors(&self) -> MacroColor;

    fn sim_lim(&self) -> usize;

    fn deconstruct_inputs(&self, slot: MacroSlot) -> Config;
    fn reconstruct_outputs(&self, config: Config) -> MacroInstr;
}

/**************************************/

struct TapeColorConverter<const base_colors: usize> {
    ct_cache: RefCell<Dict<MacroColor, Tape>>,
    tc_cache: RefCell<Dict<Tape, MacroColor>>,
}

impl<const base_colors: usize> TapeColorConverter<base_colors> {
    fn new(cells: usize) -> Self {
        let mut ct_cache = Dict::new();

        ct_cache.insert(0, vec![0; cells]);

        Self {
            ct_cache: ct_cache.into(),
            tc_cache: Dict::new().into(),
        }
    }

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
                    let base = base_colors as MacroColor;
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
#[expect(private_bounds)]
impl<const s: usize, const c: usize, L: Logic<s, c>>
    MacroProg<'_, s, c, L>
{
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
    let prog = Prog::<5, 2>::from(
        "1RB 1LC  1RC 1RB  1RD 0LE  1LA 1LD  ... 0LA",
    );

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
    let prog = Prog::<5, 2>::from(
        "0RB 0LC  1LA 1RB  1RD 0RE  1LC 1LA  ... 0LD",
    );

    let block = prog.make_block_macro(2);

    block.assert_params((10, 4));

    for &(slot, instr) in MACROS {
        assert_eq!(Ok(Some(instr)), block.get_instr(&slot));
    }
}
