use core::cmp::{max, min};

use pyo3::pyfunction;

use crate::{
    instrs::{Color, CompProg, Instr, Slot, State},
    parse::show_comp,
    tape::BasicTape as Tape,
};

type Step = u64;

/**************************************/

fn make_instrs(states: State, colors: Color) -> Vec<Instr> {
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

fn leaf(
    prog: &CompProg,
    (max_state, max_color): Params,
    harvester: &mut dyn FnMut(&CompProg),
) {
    if prog.values().all(|(_, _, state)| 1 + state < max_state)
        || prog.values().all(|(color, _, _)| 1 + color < max_color)
    {
        return;
    }

    harvester(prog);
}

#[allow(clippy::too_many_arguments)]
fn branch(
    instr: Instr,
    prog: &mut CompProg,
    (state, tape): Config,
    sim_lim: Step,
    (mut avail_states, mut avail_colors): Params,
    params: Params,
    remaining_slots: Slots,
    harvester: &mut dyn FnMut(&CompProg),
) {
    let (max_states, max_colors) = params;

    if remaining_slots == 0 {
        leaf(prog, params, harvester);
        return;
    }

    let Ok(Some(slot)) = run_for_undefined(prog, state, tape, sim_lim)
    else {
        leaf(prog, params, harvester);
        return;
    };

    let (slot_state, slot_color) = slot;
    let (instr_color, _, instr_state) = instr;

    if avail_states < max_states
        && 1 + max(slot_state, instr_state) == avail_states
    {
        avail_states += 1;
    }

    if avail_colors < max_colors
        && 1 + max(slot_color, instr_color) == avail_colors
    {
        avail_colors += 1;
    }

    for next_instr in make_instrs(avail_states, avail_colors) {
        prog.insert(slot, next_instr);

        branch(
            next_instr,
            prog,
            (slot.0, &mut tape.clone()),
            sim_lim,
            (avail_states, avail_colors),
            (max_states, max_colors),
            remaining_slots - 1,
            harvester,
        );

        prog.remove(&slot);
    }
}

fn build_tree(
    (states, colors): Params,
    halt: bool,
    sim_lim: Step,
    harvester: &mut dyn FnMut(&CompProg),
) {
    let init_slot = (0, 0);
    let init_instr = (1, true, 1);

    branch(
        init_instr,
        &mut CompProg::from([(init_slot, init_instr)]),
        (1, &mut Tape::init_stepped()),
        sim_lim,
        (min(3, states), min(3, colors)),
        (states, colors),
        (states * colors) - (1 + Slots::from(halt)),
        harvester,
    );
}

#[pyfunction]
pub fn tree_progs(
    params: Params,
    halt: bool,
    sim_lim: Step,
) -> Vec<String> {
    let mut progs = vec![];

    build_tree(params, halt, sim_lim, &mut |comp| {
        progs.push(show_comp(comp, Some(params)));
    });

    progs
}

#[test]
fn test_tree() {
    let leaves = vec![
        ((2, 2), 1, 36),
        ((2, 2), 0, 106),
        //
        ((3, 2), 1, 3_140),
        ((3, 2), 0, 13_128),
        //
        ((2, 3), 1, 2_447),
        ((2, 3), 0, 9_168),
        //
        ((4, 2), 1, 467_142),
        ((4, 2), 0, 2_291_637),
        //
        ((2, 4), 1, 312_627),
        ((2, 4), 0, 1_718_772),
        //
        ((5, 2), 1, 95_309_237),
        //
        ((2, 5), 1, 70_004_752),
    ];

    for (params, halt, leaves) in leaves {
        let mut leaf_count = 0;

        build_tree(params, halt != 0, 100, &mut |_| {
            leaf_count += 1;
        });

        assert_eq!(leaf_count, leaves);
    }
}

#[test]
fn test_progs() {
    let params = (3, 2);

    build_tree(params, true, 100, &mut |comp| {
        println!("{}", show_comp(comp, Some(params)));
    });
}
