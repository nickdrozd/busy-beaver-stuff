use core::cmp::min;

use pyo3::{
    create_exception, exceptions::PyException, pyfunction, PyResult,
};

use crate::{
    instrs::{Color, CompProg, Instr, Slot, State},
    parse::{show_comp, tcompile},
    tape::BasicTape as Tape,
};

type Step = u64;

/**************************************/

#[pyfunction]
pub fn make_instrs(states: State, colors: Color) -> Vec<Instr> {
    let mut instrs = vec![];

    let shifts = [false, true];

    for color in 0..colors {
        for shift in shifts {
            for state in 0..states {
                instrs.push((color, shift, state));
            }
        }
    }

    instrs
}

/**************************************/

create_exception!(tree, TreeSkip, PyException);

#[pyfunction]
pub fn run_for_undefined_py(
    prog: &str,
    sim_lim: Step,
) -> PyResult<Option<Slot>> {
    run_for_undefined(
        &tcompile(prog),
        1,
        &mut Tape::init_stepped(),
        sim_lim,
    )
    .map_err(|()| TreeSkip::new_err(""))
}

fn run_for_undefined(
    comp: &CompProg,
    mut state: State,
    tape: &mut Tape,
    sim_lim: Step,
) -> Result<Option<Slot>, ()> {
    for _ in 0..sim_lim {
        let slot = (state, tape.scan);

        let Some(&(color, shift, next_state)) = comp.get(&slot) else {
            return Ok(Some(slot));
        };

        let same = state == next_state;

        if same && tape.at_edge(shift) {
            return Err(());
        }

        let _ = tape.step(shift, color, same);

        if tape.blank() {
            return Err(());
        }

        state = next_state;
    }

    Ok(None)
}

/**************************************/

type Slots = u64;
type Params = (State, Color);
type Config<'t> = (State, &'t mut Tape);

#[derive(Debug)]
struct TreeProg {
    instr: Instr,
    nodes: Option<(Slot, Vec<TreeProg>)>,
}

impl TreeProg {
    const fn leaf(instr: Instr) -> Self {
        Self { instr, nodes: None }
    }

    fn branch(
        instr: Instr,
        prog: &mut CompProg,
        (state, tape): Config,
        sim_lim: Step,
        (mut avail_states, mut avail_colors): Params,
        (max_states, max_colors): Params,
        remaining_slots: Slots,
    ) -> Self {
        if remaining_slots == 0 {
            return Self::leaf(instr);
        }

        let Ok(Some(slot)) =
            run_for_undefined(prog, state, tape, sim_lim)
        else {
            return Self::leaf(instr);
        };

        let (state, color) = slot;

        if 1 + state == avail_states && avail_states < max_states {
            avail_states += 1;
        }

        if 1 + color == avail_colors && avail_colors < max_colors {
            avail_colors += 1;
        }

        let mut nodes = vec![];

        for next_instr in make_instrs(avail_states, avail_colors) {
            prog.insert(slot, next_instr);

            nodes.push(Self::branch(
                next_instr,
                prog,
                (slot.0, &mut tape.clone()),
                sim_lim,
                (avail_states, avail_colors),
                (max_states, max_colors),
                remaining_slots - 1,
            ));

            prog.remove(&slot);
        }

        assert!(!nodes.is_empty());

        Self {
            instr,
            nodes: Some((slot, nodes)),
        }
    }

    fn leaves(&self) -> usize {
        match &self.nodes {
            None => 1,
            Some((_, nodes)) => nodes.iter().map(Self::leaves).sum(),
        }
    }

    fn depth(&self) -> usize {
        1 + match &self.nodes {
            None => 0,
            Some((_, nodes)) => {
                nodes.iter().map(Self::depth).max().unwrap()
            },
        }
    }

    fn harvest<'p>(
        &self,
        params: Params,
        slot: Slot,
        prog: &mut CompProg,
        progs: &'p mut Vec<String>,
    ) -> &'p mut Vec<String> {
        prog.insert(slot, self.instr);

        if let Some((next_slot, nodes)) = &self.nodes {
            for node in nodes {
                node.harvest(params, *next_slot, prog, progs);
            }
        } else {
            progs.push(show_comp(prog, Some(params)));
        };

        prog.remove(&slot);

        progs
    }
}

fn build_tree(
    (states, colors): Params,
    halt: bool,
    sim_lim: Step,
) -> TreeProg {
    let init_slot = (0, 0);
    let init_instr = (1, true, 1);

    TreeProg::branch(
        init_instr,
        &mut CompProg::from([(init_slot, init_instr)]),
        (1, &mut Tape::init_stepped()),
        sim_lim,
        (min(3, states), min(3, colors)),
        (states, colors),
        (states * colors) - (1 + Slots::from(halt)),
    )
}

#[pyfunction]
pub fn tree_progs(
    params: Params,
    halt: bool,
    sim_lim: Step,
) -> Vec<String> {
    build_tree(params, halt, sim_lim)
        .harvest(
            params,
            (1, 0),
            &mut CompProg::from([((0, 0), (1, true, 1))]),
            &mut vec![],
        )
        .clone()
}

#[test]
fn test_tree() {
    let leaves = vec![
        ((2, 2, 1), 36),
        ((2, 2, 0), 106),
        ((3, 2, 1), 3_246),
        ((3, 2, 0), 13_234),
        ((2, 3, 1), 2_553),
        ((2, 3, 0), 9_274),
        ((4, 2, 1), 480_376),
        ((4, 2, 0), 2_304_871),
        ((2, 4, 1), 196_668),
        ((2, 4, 0), 1_034_073),
    ];

    for ((states, colors, halt), leaves) in leaves {
        let tree = build_tree((states, colors), halt != 0, 100);

        assert_eq!(tree.leaves(), leaves, "{states} {colors} {halt}");
        assert_eq!(tree.depth(), ((states * colors) - halt) as usize);
    }
}
