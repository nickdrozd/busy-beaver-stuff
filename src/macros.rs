use core::ops::Index;
use std::collections::HashMap;
use std::sync::Mutex;

use lazy_static::lazy_static;

use crate::instrs::{Color, Instr, Slot, State};

type Tape = Vec<Color>;
type Config = (State, (bool, Tape));

/**************************************/

#[derive(Clone)]
struct TapeColorConverter {
    base_colors: Color,
    color_to_tape_cache: HashMap<Color, Tape>,
    tape_to_color_cache: HashMap<Tape, Color>,
}

impl TapeColorConverter {
    fn new(base_colors: Color, cells: usize) -> Self {
        let mut color_to_tape = HashMap::new();
        color_to_tape.insert(0, vec![0; cells]);

        Self {
            base_colors,
            color_to_tape_cache: color_to_tape,
            tape_to_color_cache: HashMap::new(),
        }
    }

    fn color_to_tape(&self, color: Color) -> Tape {
        self.color_to_tape_cache[&color].clone()
    }

    fn tape_to_color(&mut self, tape: Tape) -> Color {
        if let Some(color) = self.tape_to_color_cache.get(&tape) {
            return *color;
        }

        let color =
            tape.iter().rev().enumerate().fold(0, |acc, (i, &val)| {
                acc + val * self.base_colors.pow(i as u32)
            });

        self.tape_to_color_cache.insert(tape.clone(), color);
        self.color_to_tape_cache.insert(color, tape);

        color
    }
}

type ConverterCache =
    HashMap<Color, HashMap<usize, TapeColorConverter>>;

lazy_static! {
    static ref CONVERTERS: Mutex<ConverterCache> =
        Mutex::new(HashMap::new());
}

fn make_converter(
    base_colors: Color,
    cells: usize,
) -> TapeColorConverter {
    CONVERTERS
        .lock()
        .unwrap()
        .entry(base_colors)
        .or_default()
        .entry(cells)
        .or_insert_with(|| TapeColorConverter::new(base_colors, cells))
        .clone()
}

/**************************************/

type GetInstr = HashMap<(State, Color), Instr>;

#[derive(Debug, Clone)]
struct MacroInfLoop;

type MacroResult<T> = Result<T, MacroInfLoop>;

trait MacroProg: Index<Slot, Output = Instr> + Sized {
    fn comp(&self) -> &GetInstr;
    fn instrs(&self) -> &HashMap<Slot, Instr>;

    fn base_states(&self) -> usize;
    fn base_colors(&self) -> usize;

    fn macro_states(&self) -> usize;
    fn macro_colors(&self) -> usize;

    fn sim_lim(&self) -> usize;

    fn deconstruct_inputs(
        &self,
        macro_state: State,
        macro_color: Color,
    ) -> Config;

    fn reconstruct_outputs(&self, config: Config) -> Instr;

    fn calculate_instr(
        &self,
        macro_state: State,
        macro_color: Color,
    ) -> MacroResult<Instr> {
        Ok(self.reconstruct_outputs(self.run_simulator(
            self.deconstruct_inputs(macro_state, macro_color),
        )?))
    }

    fn run_simulator(&self, config: Config) -> MacroResult<Config> {
        let (mut state, (right_edge, mut tape)) = config;

        let cells = tape.len();

        let mut pos = if right_edge { cells - 1 } else { 0 };

        let mut side: Option<bool> = None;

        'step: for _ in 0..self.sim_lim() {
            let scan = tape[pos];

            let (color, shift, next_state) =
                self.comp()[&(state, scan)];

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
            } else {
                #[allow(clippy::collapsible_else_if)]
                if shift {
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
        }

        Ok((state, (side.ok_or(MacroInfLoop)?, tape)))
    }
}

/**************************************/

struct BlockMacro {
    cells: usize,
    base_states: State,
    base_colors: Color,
}

/**************************************/

struct BacksymbolMacro {
    cells: usize,
    base_states: usize,
    base_colors: usize,
}

impl BacksymbolMacro {
    const fn backsymbols(&self) -> usize {
        self.base_colors.pow(self.cells as u32)
    }
}
