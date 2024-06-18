use std::collections::{HashMap, HashSet};

use pyo3::pyfunction;

use crate::{
    instrs::{Color, CompProg, Instr, Shift, State},
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

type Step = u64;

fn cant_reach(prog: &str, term_type: TermType) -> bool {
    let mut configs: Vec<(Step, State, Tape)> = match term_type {
        TermType::Halt => halt_configs,
        TermType::Blank => erase_configs,
        TermType::Spinout => zero_reflexive_configs,
    }(prog)
    .into_iter()
    .map(|(state, tape)| (1, state, tape))
    .collect();

    if configs.is_empty() {
        return true;
    }

    let comp = tcompile(prog);

    let max_steps = 24;
    let max_cycles = 94;

    let (colors, entry_points, program) = rparse(prog);

    let mut seen: HashMap<State, HashSet<Tape>> = HashMap::new();

    let run = match term_type {
        TermType::Halt => run_halt,
        TermType::Blank => run_blank,
        TermType::Spinout => run_spinout,
    };

    for _ in 0..max_cycles {
        let Some((step, state, tape)) = configs.pop() else {
            return true;
        };

        let next_step = 1 + step;

        if next_step > max_steps {
            return false;
        }

        if seen.entry(state).or_default().contains(&tape) {
            continue;
        }

        seen.get_mut(&state).unwrap().insert(tape.clone());

        if state == 0 && tape.blank() {
            return false;
        }

        // println!("{step} | {state} | {tape}");

        for (next_color, &next_state) in
            entry_points[&state].iter().enumerate()
        {
            for &(_, shift, trans) in &program[&next_state] {
                if trans != state {
                    continue;
                }

                for try_color in 0..colors as Color {
                    let Some(&(prev_color, come_back, prev_state)) =
                        comp.get(&(next_state, try_color))
                    else {
                        continue;
                    };

                    if come_back != shift || prev_state != state {
                        continue;
                    }

                    let mut next_tape = tape.clone();

                    let overwrite =
                        next_tape.backstep(shift, try_color);

                    #[allow(clippy::branches_sharing_code)]
                    if state == next_state
                        && try_color == next_color as Color
                    {
                        if prev_color == overwrite {
                            return false;
                        }

                        next_tape.step(come_back, prev_color, false);

                        if next_tape != tape {
                            continue;
                        }

                        next_tape.backstep(shift, try_color);
                    } else {
                        if !validate(
                            &comp, next_step, next_tape, next_state,
                            &run,
                        ) {
                            continue;
                        }

                        next_tape = tape.clone();
                        next_tape.backstep(shift, try_color);
                    }

                    configs.push((next_step, next_state, next_tape));
                }
            }
        }
    }

    false
}

/**************************************/

type Config = (State, Tape);

fn halt_configs(prog: &str) -> Vec<Config> {
    parse(prog)
        .enumerate()
        .flat_map(|(state, instrs)| {
            instrs.enumerate().filter_map(move |(color, instr)| {
                instr.is_none().then_some((
                    state as State,
                    Tape::init(color as Color),
                ))
            })
        })
        .collect()
}

fn erase_configs(prog: &str) -> Vec<Config> {
    parse(prog)
        .enumerate()
        .flat_map(|(state, instrs)| {
            instrs.enumerate().filter_map(move |(color, instr)| {
                match instr {
                    Some((0, _, _)) if color != 0 => Some((
                        state as State,
                        Tape::init(color as Color),
                    )),
                    _ => None,
                }
            })
        })
        .collect()
}

fn zero_reflexive_configs(prog: &str) -> Vec<Config> {
    parse(prog)
        .enumerate()
        .filter_map(|(state, mut instrs)| {
            instrs.next().and_then(|instr| {
                if let Some((_, _, tr)) = instr {
                    if tr == state as State {
                        return Some((state as State, Tape::init(0)));
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
    fn backstep(&mut self, shift: Shift, color: Color) -> Color;
}

impl Backstep for Tape {
    fn backstep(&mut self, shift: Shift, color: Color) -> Color {
        let _ = self.step(!shift, self.scan, false);

        let overwrite = self.scan;

        self.scan = color;

        overwrite
    }
}

/**************************************/

fn validate(
    comp: &CompProg,
    step: Step,
    tape: Tape,
    state: State,
    run: &impl Fn(&CompProg, Step, Tape, State) -> Option<Step>,
) -> bool {
    let Some(result) = run(comp, step, tape, state) else {
        return false;
    };

    (result as isize - step as isize).abs() <= 1
}

fn run_halt(
    comp: &CompProg,
    sim_lim: Step,
    mut tape: Tape,
    mut state: State,
) -> Option<Step> {
    let mut step = 0;

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

fn run_blank(
    comp: &CompProg,
    sim_lim: Step,
    mut tape: Tape,
    mut state: State,
) -> Option<Step> {
    let mut blanks: HashMap<State, Step> = HashMap::new();

    let mut step = 0;

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

fn run_spinout(
    comp: &CompProg,
    sim_lim: Step,
    mut tape: Tape,
    mut state: State,
) -> Option<Step> {
    let mut step = 0;

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
