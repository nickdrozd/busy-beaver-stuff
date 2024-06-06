use std::collections::{HashMap, HashSet};

use pyo3::pyfunction;

use crate::{
    instrs::{Color, CompProg, Instr, Shift, Slot, State},
    parse::{parse, parse_to_vec, tcompile},
    tape::BasicTape as Tape,
};

/**************************************/

#[derive(Clone, Copy)]
enum TermType {
    Halt,
    Blank,
    Spinout,
}

#[pyfunction]
pub fn cant_halt(prog: &str) -> bool {
    cant_reach(prog, TermType::Halt)
}

#[pyfunction]
pub fn cant_blank(prog: &str) -> bool {
    cant_reach(prog, TermType::Blank)
}

#[pyfunction]
pub fn cant_spin_out(prog: &str) -> bool {
    cant_reach(prog, TermType::Spinout)
}

/**************************************/

fn cant_reach(prog: &str, term_type: TermType) -> bool {
    let mut configs: Vec<(Color, State, Tape)> = match term_type {
        TermType::Halt => halt_slots,
        TermType::Blank => erase_slots,
        TermType::Spinout => zero_reflexive_slots,
    }(prog)
    .iter()
    .map(|&(state, color)| (1, state, Tape::init(color)))
    .collect();

    if configs.is_empty() {
        return true;
    }

    let comp = tcompile(prog);

    let max_steps = 24;
    let max_cycles = 1_000;

    let (colors, entry_points, program) = rparse(prog);

    let mut seen: HashMap<State, HashSet<Tape>> = HashMap::new();

    let backstep_run = match term_type {
        TermType::Halt => backstep_run_halt,
        TermType::Blank => backstep_run_blank,
        TermType::Spinout => backstep_run_spinout,
    };

    for _ in 0..max_cycles {
        let Some((step, state, tape)) = configs.pop() else {
            return true;
        };

        if step > max_steps {
            return false;
        }

        if state == 0 && tape.blank() {
            return false;
        }

        if seen.entry(state).or_default().contains(&tape) {
            continue;
        }

        seen.get_mut(&state).unwrap().insert(tape.clone());

        // println!("{step} | {state} | {tape}");

        let next_step = 1 + step;

        for entry in &entry_points[&state] {
            let next_state = *entry;

            for &(_, shift, trans) in &program[entry] {
                if trans != state {
                    continue;
                }

                for color in 0..colors {
                    let Some(result) = backstep_run(
                        &comp,
                        next_step,
                        tape.clone(),
                        next_state,
                        shift,
                        color as Color,
                    ) else {
                        continue;
                    };

                    if (result as isize - step as isize).abs() > 1 {
                        continue;
                    }

                    let mut next_tape = tape.clone();

                    next_tape.backstep(shift, color as Color);

                    configs.push((next_step, next_state, next_tape));
                }
            }
        }
    }

    false
}

/**************************************/

fn halt_slots(prog: &str) -> Vec<Slot> {
    parse(prog)
        .enumerate()
        .flat_map(|(state, instrs)| {
            instrs.enumerate().filter_map(move |(color, instr)| {
                instr
                    .is_none()
                    .then_some((state as State, color as Color))
            })
        })
        .collect()
}

fn erase_slots(prog: &str) -> Vec<Slot> {
    parse(prog)
        .enumerate()
        .flat_map(|(state, instrs)| {
            instrs.enumerate().filter_map(move |(color, instr)| {
                match instr {
                    Some((0, _, _)) if color != 0 => {
                        Some((state as State, color as Color))
                    },
                    _ => None,
                }
            })
        })
        .collect()
}

fn zero_reflexive_slots(prog: &str) -> Vec<Slot> {
    parse(prog)
        .enumerate()
        .filter_map(|(state, mut instrs)| {
            instrs.next().and_then(|instr| {
                if let Some((_, _, tr)) = instr {
                    if tr == state as State {
                        return Some((state as State, 0));
                    }
                }
                None
            })
        })
        .collect()
}

/**************************************/

type Graph = HashMap<State, Vec<State>>;
type Program = HashMap<State, Vec<Instr>>;

fn entry_points(program: &Program) -> Graph {
    let mut exits: HashMap<State, HashSet<State>> = HashMap::new();

    for (state, instrs) in program {
        exits.insert(
            *state,
            instrs.iter().map(|instr| instr.2).collect(),
        );
    }

    let mut entries: Graph = (0..program.len())
        .map(|state| (state as State, Vec::new()))
        .collect();

    for (state, cons) in exits {
        for exit_point in cons {
            if let Some(states) = entries.get_mut(&exit_point) {
                states.push(state);
            }
        }
    }

    for entr in entries.values_mut() {
        entr.sort_unstable();
    }

    entries
}

fn rparse(prog: &str) -> (usize, Graph, Program) {
    let mut program = Program::new();

    let parsed = parse_to_vec(prog);

    for (state, instrs) in parsed.iter().enumerate() {
        program.insert(
            state as State,
            instrs.iter().filter_map(|instr| *instr).collect(),
        );
    }

    (parsed[0].len(), entry_points(&program), program)
}

/**************************************/

trait Backstep {
    fn backstep(&mut self, shift: Shift, color: Color);
}

impl Backstep for Tape {
    fn backstep(&mut self, shift: Shift, color: Color) {
        let _ = self.step(!shift, self.scan, false);

        self.scan = color;
    }
}

/**************************************/

type Step = u64;

fn backstep_run_halt(
    comp: &CompProg,
    sim_lim: Step,
    mut tape: Tape,
    mut state: State,
    shift: Shift,
    color: Color,
) -> Option<Step> {
    let mut step = 0;

    tape.backstep(shift, color);

    for _ in 0..sim_lim {
        let Some(&(color, shift, next_state)) =
            comp.get(&(state, tape.scan))
        else {
            return Some(step);
        };

        let same = state == next_state;

        if same && tape.at_edge(shift) {
            break;
        }

        let stepped = tape.step(shift, color, same);

        step += stepped;

        state = next_state;
    }

    None
}

fn backstep_run_blank(
    comp: &CompProg,
    sim_lim: Step,
    mut tape: Tape,
    mut state: State,
    shift: Shift,
    color: Color,
) -> Option<Step> {
    let mut blanks: HashMap<State, Step> = HashMap::new();

    let mut step = 0;

    tape.backstep(shift, color);

    for _ in 0..sim_lim {
        let Some(&(color, shift, next_state)) =
            comp.get(&(state, tape.scan))
        else {
            break;
        };

        let same = state == next_state;

        if same && tape.at_edge(shift) {
            break;
        }

        let stepped = tape.step(shift, color, same);

        step += stepped;

        state = next_state;

        if color == 0 && tape.blank() {
            if blanks.contains_key(&state) {
                break;
            }

            blanks.insert(state, step);

            if state == 0 {
                break;
            }
        }
    }

    blanks.drain().map(|(_, value)| value).min()
}

fn backstep_run_spinout(
    comp: &CompProg,
    sim_lim: Step,
    mut tape: Tape,
    mut state: State,
    shift: Shift,
    color: Color,
) -> Option<Step> {
    let mut step = 0;

    tape.backstep(shift, color);

    for _ in 0..sim_lim {
        let &(color, shift, next_state) =
            comp.get(&(state, tape.scan))?;

        let same = state == next_state;

        if same && tape.at_edge(shift) {
            return Some(step);
        }

        let stepped = tape.step(shift, color, same);

        step += stepped;

        state = next_state;
    }

    None
}
