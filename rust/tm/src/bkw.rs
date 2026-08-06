use core::{
    fmt,
    hash::{Hash, Hasher as _},
    iter::once,
};

use ahash::{AHashMap as Dict, AHashSet as Set, AHasher};

use crate::{
    Color, Instr, Prog, Shift, Slot, State, Steps,
    instrs::Parse as _,
    tape::{Pos, Scan},
};

const MAX_STACK_DEPTH: usize = 64;

/**************************************/

#[derive(Debug)]
pub enum BackwardResult {
    Init,
    StepLimit,
    DepthLimit,
    Refuted(Steps),
}

use BackwardResult::*;

impl BackwardResult {
    pub const fn is_refuted(&self) -> bool {
        matches!(self, Refuted(_))
    }
}

/**************************************/

impl<const s: usize, const c: usize> Prog<s, c> {
    pub fn bkw_cant_halt(&self, steps: Steps) -> BackwardResult {
        let (entrypoints, idx) = self.entrypoints_and_indices();

        let slots = self.halt_slots_disp_side(&idx);
        let slots = self.halt_slots_side_excursion(slots);

        cant_reach(self, steps, slots, Some(entrypoints), halt_configs)
    }

    pub fn bkw_cant_blank(&self, steps: Steps) -> BackwardResult {
        if self.cant_blank_by_color_graph() {
            return Refuted(0);
        }

        cant_reach(
            self,
            steps,
            self.blank_slots_side_clean(),
            None,
            erase_configs,
        )
    }

    pub fn bkw_cant_spinout(&self, steps: Steps) -> BackwardResult {
        cant_reach(
            self,
            steps,
            self.spinout_shifts_side_clean(),
            None,
            zr_configs,
        )
    }

    pub fn bkw_cant_zloop(&self, steps: Steps) -> BackwardResult {
        cant_reach(
            self,
            steps,
            self.zloop_shifts_side_clean(),
            None,
            zr_configs,
        )
    }

    pub fn bkw_cant_twostep(&self, steps: Steps) -> BackwardResult {
        cant_reach(
            self,
            steps,
            self.twostep_slots()
                .into_iter()
                .map(|((st, l_co), (_, r_co))| (st, (l_co, r_co)))
                .collect(),
            None,
            twostep_configs,
        )
    }
}

/**************************************/

type Configs = Vec<Config>;
type BlankStates = Set<State>;

type Entry = (Slot, (Color, Shift));
type Entries = Vec<Entry>;
type Entrypoints = Dict<State, (Entries, Entries)>;

/// Compact lookup tables for which 3-cell windows `(L, scan, R)` are
/// possible in some run from the blank tape.
///
/// - `right[st][scan][left]` is a bitmask of possible right colors.
/// - `left[st][scan][right]` is a bitmask of possible left colors.
/// - `any[st][scan]` records whether at least one neighbor pair is possible.
///
/// This makes all four known/unknown-neighbor lookup cases constant-time.
struct WinPossible<const S: usize, const C: usize> {
    right: [[[u64; C]; C]; S],
    left: [[[u64; C]; C]; S],
    any: [[bool; C]; S],
}

const LEFT_SIDE: usize = 0;
const RIGHT_SIDE: usize = 1;

/// State-aware over-approximation of every color and adjacent color pair that
/// can occur on either whole side of the head in a run from the blank tape.
///
/// Pairs are oriented from the head toward the tape end.  Thus
/// `pairs[state][side][near]` is a bitmask of possible `far` colors directly
/// adjacent to `near` somewhere on that side.  The infinite blank tail is part
/// of each side, so reachable states always admit color 0 and pair (0, 0).
struct SidePossible<const S: usize, const C: usize> {
    colors: [[u64; 2]; S],
    pairs: [[[u64; C]; 2]; S],
}

/// Bit `p` of `possible[state]` is set when the transition graph admits a
/// run from the blank initial configuration to `state` with
/// `p == (# nonblank tape cells mod 2)`.
///
/// This deliberately forgets the tape contents and therefore computes an
/// over-approximation. A missing bit is nevertheless a sound invariant and
/// can be used to prune backward configurations whose complete finite tape has
/// the wrong parity.
struct NonblankParity<const S: usize> {
    possible: [u8; S],
}

fn cant_reach<const s: usize, const c: usize, T: Ord>(
    prog: &Prog<s, c>,
    steps: Steps,
    mut slots: Set<(State, T)>,
    entrypoints: Option<Entrypoints>,
    get_configs: impl Fn(&Set<(State, T)>) -> Configs,
) -> BackwardResult {
    if slots.is_empty() {
        return Refuted(0);
    }

    let entrypoints =
        entrypoints.unwrap_or_else(|| prog.get_entrypoints());

    slots.retain(|(state, _)| entrypoints.contains_key(state));

    if slots.is_empty() {
        return Refuted(0);
    }

    // Shift-side analysis:
    // For some colors, the transition table itself proves they can
    // never appear on one side of the head in any run from the blank
    // tape. (Example: if a color is never written on an L-move, it
    // cannot persist to the right of the head.) We use this as a
    // *sound* pruning filter to avoid spurious backward
    // configurations.
    let (forbid_left, forbid_right) = prog.shift_side_forbidden();

    // If shift-side analysis proves that *no non-blank* symbol can ever
    // appear on a given side of the head in any run from the blank
    // tape, then that entire side is forced to be blank.
    //
    // This remains sound even if the program *can* print blank: the
    // invariant is about which symbols can occur on each side, not
    // about whether a cell has been visited.
    let left_forced_blank = (1..c).all(|k| forbid_left[k]);
    let right_forced_blank = (1..c).all(|k| forbid_right[k]);

    // One-sided blank-write analysis (strictly stronger than the global
    // "never writes blank" special case).
    //
    // For a cell to contain blank (0) *within the visited region* on the
    // left of the head, the last time that cell was visited the head must
    // have moved Right after writing 0 there. Therefore, if the program
    // never writes 0 on a Right move, any 0 appearing to the *left* of the
    // head must be unvisited, and thus all cells farther left must also be
    // unvisited blanks. Symmetrically for the right side and Left moves.
    let (writes_blank_on_r, writes_blank_on_l) =
        prog.blank_writes_by_shift();
    let left_fresh_zero = !writes_blank_on_r;
    let right_fresh_zero = !writes_blank_on_l;

    // Sound state/nonblank-count parity invariant. This is especially useful
    // after the fresh-zero rules turn a formerly unknown tape end into `0+`,
    // making the total nonblank parity exact.
    let nonblank_parity = prog.nonblank_parity_from_blank();

    let mut configs = get_configs(&slots);

    // Apply the cheap static side filters before constructing the window
    // fixed point. This preserves the old short-circuit order while avoiding
    // window construction when every target is already impossible.
    configs.retain_mut(|Config { tape, .. }| {
        tape.obeys_shift_side(&forbid_left, &forbid_right)
            && tape.tighten_forced_blank_ends(
                left_forced_blank,
                right_forced_blank,
            )
            && tape.enforce_fresh_zero_side_invariants(
                left_fresh_zero,
                right_fresh_zero,
            )
    });

    configs.retain(|Config { state, tape }| {
        nonblank_parity_possible(*state, tape, &nonblank_parity)
    });

    if configs.is_empty() {
        return Refuted(0);
    }

    // Optional *sound* adjacency reachability filter.
    //
    // We over-approximate the set of 3-cell windows (L,scan,R) that
    // can appear around the head in each state when starting from the
    // blank tape. If a generated predecessor configuration demands an
    // immediate neighbor color that is impossible in this
    // over-approximation, we can safely prune it.
    let win_possible =
        prog.win_possible_from_blank(&forbid_left, &forbid_right);
    let side_possible = prog.side_possible_from_blank(&win_possible);

    // Halt targets begin with two unknown neighbors, so the
    // `(state, scanned color)` pair must still occur in at least one reachable
    // window after the cheaper side filters above have canonicalized the tape.
    configs.retain(|Config { state, tape }| {
        window_possible(*state, tape, &win_possible)
            && tape.obeys_state_side(*state, &side_possible)
    });

    if configs.is_empty() {
        return Refuted(0);
    }

    let mut blanks = get_blanks(&configs);

    let mut antichains = Antichains::default();

    // Periodic branch closers.  These are separate histories over different
    // streams, but they use the same periodic-growth certificate: the linear
    // closer observes one-config snapshots, while the frontier closer observes
    // the whole live frontier after antichain filtering.
    let mut periodic_history = PeriodicHistory::default();
    let mut frontier_periodic_history = PeriodicHistory::default();
    let mut coverage_periodic_history =
        CoveragePeriodicHistory::default();

    let mut seen: Set<(State, u64)> = Set::new();

    for step in 1..=steps {
        configs.retain(|Config { state, tape }| {
            let blank_ends = tape.lspan.end == TapeEnd::Blanks
                && tape.rspan.end == TapeEnd::Blanks;

            !blank_ends || seen.insert((*state, tape.hash()))
        });

        #[cfg(debug_assertions)]
        {
            for config in &configs {
                println!("{step} | {config}");
            }
            println!();
        };

        let valid_steps = get_valid_steps(&mut configs, &entrypoints);

        match valid_steps.len() {
            0 => return Refuted(step),
            n if MAX_STACK_DEPTH < n => return DepthLimit,
            _ => {},
        }

        let linear_before_step =
            valid_steps.len() == 1 && valid_steps[0].0.len() == 1;

        let stepped = match step_configs::<s, c>(
            valid_steps,
            &mut blanks,
            &win_possible,
            &side_possible,
            &forbid_left,
            &forbid_right,
            left_fresh_zero,
            right_fresh_zero,
            left_forced_blank,
            right_forced_blank,
            &nonblank_parity,
        ) {
            Err(err) => return err,
            Ok(stepped) => stepped,
        };

        let close_linear_periodic = linear_before_step
            && stepped.len() == 1
            && periodic_history.push_and_detect(&stepped[0]);

        if close_linear_periodic {
            configs.clear();
        } else {
            if !linear_before_step || stepped.len() != 1 {
                periodic_history.clear();
            }

            configs = stepped
                .into_iter()
                .filter(|config| antichains.insert(config))
                .collect();
        }

        // `configs` is the live frontier that will be printed/processed at
        // backward depth `step + 1`, after antichain filtering.
        //
        // Both periodic closers consume the same canonical FastCfg snapshot.
        // Build and sort it once, then share the immutable allocation between
        // their separate histories.  Previously each closer converted and
        // sorted the whole frontier independently.
        if configs.len()
            > CoveragePeriodicHistory::MAX_FRONTIER_FOR_CLOSER
        {
            frontier_periodic_history.clear();
            coverage_periodic_history.clear();
        } else if !configs.is_empty() {
            let fast_front = sorted_fast_frontier(&configs);

            if let Some(cycle_from) = frontier_periodic_history
                .observe_frontier(step + 1, Arc::clone(&fast_front))
            {
                return Refuted(cycle_from);
            }

            // Broader certificate for mixed frontiers.  This does not prune
            // individual branches and does not replace precise configs with
            // `..`; it only refutes after every config in each later frontier
            // is covered by a repeated stable/growth relation from the
            // matching earlier phase.  Unlike the exact frontier checker,
            // extra later configs are allowed, but only when they are covered
            // by the same phase relations.
            if let Some(cycle_from) = coverage_periodic_history
                .observe_frontier(step + 1, fast_front)
            {
                return Refuted(cycle_from);
            }
        }
    }

    StepLimit
}

type ValidatedSteps = Vec<(Vec<Instr>, Config)>;

fn get_valid_steps(
    configs: &mut Configs,
    entrypoints: &Entrypoints,
) -> ValidatedSteps {
    let mut checked = ValidatedSteps::with_capacity(configs.len());

    for config in configs.drain(..) {
        let Config { state, tape } = &config;

        let Some((same, diff)) = entrypoints.get(state) else {
            assert_eq!(*state, 0);
            continue;
        };

        let mut steps = Vec::with_capacity(same.len() + diff.len());

        for &((next_state, color), (print, shift)) in diff {
            if !tape.is_valid_step(shift, print) {
                continue;
            }

            steps.push((color, shift, next_state));
        }

        for &((_, color), (print, shift)) in same {
            if !tape.is_valid_step(shift, print) {
                continue;
            }

            if !tape.is_spinout(shift, color) {
                steps.push((color, shift, *state));
                continue;
            }

            if let Some(indef) = get_indef(shift, &config, diff, same) {
                checked.push(indef);
            }
        }

        if steps.is_empty() {
            continue;
        }

        checked.push((steps, config));
    }

    checked
}

fn get_indef(
    push: Shift,
    config: &Config,
    diff: &Entries,
    same: &Entries,
) -> Option<(Vec<Instr>, Config)> {
    let mut tape = config.tape.clone();
    tape.push_indef(push);

    // Avoid cloning `diff` and constructing a temporary combined entry list.
    // Preserve the original order: different-state entries first, followed by
    // eligible same-state entries.
    let same_entries =
        same.iter().copied().filter(|&((_, color), (_, shift))| {
            shift != push || color != config.tape.scan
        });
    let mut steps = Vec::with_capacity(diff.len() + same.len());

    for ((state, color), (print, shift)) in
        diff.iter().copied().chain(same_entries)
    {
        if tape.is_valid_step(shift, print) {
            steps.push((color, shift, state));
        }
    }

    if steps.is_empty() {
        return None;
    }

    let next_config = Config::new(config.state, tape);

    #[cfg(debug_assertions)]
    println!("~ | {next_config}");

    Some((steps, next_config))
}

fn window_possible<const s: usize, const c: usize>(
    state: State,
    tape: &Tape,
    win_possible: &WinPossible<s, c>,
) -> bool {
    let st = state as usize;
    let sc = tape.scan as usize;

    let l = tape.left_neighbor_color().map(|x| x as usize);
    let r = tape.right_neighbor_color().map(|x| x as usize);

    match (l, r) {
        (Some(lc), Some(rc)) => {
            (win_possible.right[st][sc][lc] & (1_u64 << rc)) != 0
        },
        (Some(lc), None) => win_possible.right[st][sc][lc] != 0,
        (None, Some(rc)) => win_possible.left[st][sc][rc] != 0,
        (None, None) => win_possible.any[st][sc],
    }
}

fn window_neighbor_mask<const S: usize, const C: usize>(
    state: usize,
    scan: usize,
    shift: Shift,
    possible: &WinPossible<S, C>,
) -> u64 {
    if shift {
        possible.right[state][scan]
            .iter()
            .copied()
            .fold(0, |mask, colors| mask | colors)
    } else {
        possible.left[state][scan]
            .iter()
            .copied()
            .fold(0, |mask, colors| mask | colors)
    }
}

fn nonblank_parity_possible<const s: usize>(
    state: State,
    tape: &Tape,
    parity: &NonblankParity<s>,
) -> bool {
    let st = state as usize;

    st < s && (parity.possible[st] & tape.nonblank_parity_mask()) != 0
}

#[expect(clippy::fn_params_excessive_bools, clippy::too_many_arguments)]
fn step_instrs<const s: usize, const c: usize>(
    instrs: impl IntoIterator<Item = Instr>,
    config: &Config,
    blanks: &mut BlankStates,
    win_possible: &WinPossible<s, c>,
    side_possible: &SidePossible<s, c>,
    forbid_left: &[bool; c],
    forbid_right: &[bool; c],
    left_fresh_zero: bool,
    right_fresh_zero: bool,
    left_forced_blank: bool,
    right_forced_blank: bool,
    nonblank_parity: &NonblankParity<s>,
    stepped: &mut Configs,
) -> Result<(), BackwardResult> {
    for (color, shift, state) in instrs {
        let mut tape = config.tape.clone();

        tape.backstep(shift, color);

        if tape.blank() {
            if state == 0 {
                return Err(Init);
            }

            if !blanks.insert(state) {
                continue;
            }
        }

        // Retain the original full-span static checks. This helper only avoids
        // the temporary `branch_indef` frontier and its instruction vectors.
        if !tape.obeys_shift_side(forbid_left, forbid_right) {
            continue;
        }

        if !tape.tighten_forced_blank_ends(
            left_forced_blank,
            right_forced_blank,
        ) {
            continue;
        }

        if (left_fresh_zero || right_fresh_zero)
            && !tape.enforce_fresh_zero_side_invariants(
                left_fresh_zero,
                right_fresh_zero,
            )
        {
            continue;
        }

        if !nonblank_parity_possible(state, &tape, nonblank_parity) {
            continue;
        }

        if !window_possible(state, &tape, win_possible)
            || !tape.obeys_state_side(state, side_possible)
        {
            continue;
        }

        stepped.push(Config::new(state, tape));
    }

    Ok(())
}

#[expect(clippy::fn_params_excessive_bools, clippy::too_many_arguments)]
fn step_configs<const s: usize, const c: usize>(
    configs: ValidatedSteps,
    blanks: &mut BlankStates,
    win_possible: &WinPossible<s, c>,
    side_possible: &SidePossible<s, c>,
    forbid_left: &[bool; c],
    forbid_right: &[bool; c],
    left_fresh_zero: bool,
    right_fresh_zero: bool,
    left_forced_blank: bool,
    right_forced_blank: bool,
    nonblank_parity: &NonblankParity<s>,
) -> Result<Configs, BackwardResult> {
    let mut stepped = Configs::new();

    for (instrs, config) in configs {
        // Fuse `branch_indef` into stepping. The old processing order is
        // preserved: left count-one branch, right count-one branch, original.
        let split_left = config.tape.pull_needs_count_one_split(true);
        let split_right = config.tape.pull_needs_count_one_split(false);

        if split_left && instrs.iter().any(|&(_, shift, _)| shift) {
            let mut count_1 = config.clone();
            count_1.tape.lspan.set_head_to_one();

            step_instrs(
                instrs.iter().copied().filter(|&(_, shift, _)| shift),
                &count_1,
                blanks,
                win_possible,
                side_possible,
                forbid_left,
                forbid_right,
                left_fresh_zero,
                right_fresh_zero,
                left_forced_blank,
                right_forced_blank,
                nonblank_parity,
                &mut stepped,
            )?;
        }

        if split_right && instrs.iter().any(|&(_, shift, _)| !shift) {
            let mut count_1 = config.clone();
            count_1.tape.rspan.set_head_to_one();

            step_instrs(
                instrs.iter().copied().filter(|&(_, shift, _)| !shift),
                &count_1,
                blanks,
                win_possible,
                side_possible,
                forbid_left,
                forbid_right,
                left_fresh_zero,
                right_fresh_zero,
                left_forced_blank,
                right_forced_blank,
                nonblank_parity,
                &mut stepped,
            )?;
        }

        step_instrs(
            instrs,
            &config,
            blanks,
            win_possible,
            side_possible,
            forbid_left,
            forbid_right,
            left_fresh_zero,
            right_fresh_zero,
            left_forced_blank,
            right_forced_blank,
            nonblank_parity,
            &mut stepped,
        )?;
    }

    Ok(stepped)
}

/**************************************/

fn halt_configs(halt_slots: &Set<Slot>) -> Configs {
    halt_slots
        .iter()
        .map(|&(state, color)| Config::init_halt(state, color))
        .collect()
}

fn erase_configs(erase_slots: &Set<Slot>) -> Configs {
    erase_slots
        .iter()
        .map(|&(state, color)| Config::init_blank(state, color))
        .collect()
}

fn zr_configs(zr_shifts: &Set<(State, Shift)>) -> Configs {
    zr_shifts
        .iter()
        .map(|&(state, shift)| Config::init_spinout(state, shift))
        .collect()
}

fn twostep_configs(twosteps: &Set<(State, (Color, Color))>) -> Configs {
    twosteps
        .iter()
        .map(|&(st, (l_co, r_co))| Config::init_twostep(st, l_co, r_co))
        .collect()
}

fn get_blanks(configs: &Configs) -> BlankStates {
    configs
        .iter()
        .filter_map(|cfg| cfg.tape.blank().then_some(cfg.state))
        .collect()
}

/**************************************/

#[expect(clippy::multiple_inherent_impl)]
impl<const s: usize, const c: usize> Prog<s, c> {
    fn get_entrypoints(&self) -> Entrypoints {
        let mut entrypoints = Entrypoints::new();

        for (slot @ (read, _), &(color, shift, state)) in self.iter() {
            let (same, diff) = entrypoints.entry(state).or_default();

            (if read == state { same } else { diff })
                .push((slot, (color, shift)));
        }

        entrypoints
    }

    /// Returns (writes_blank_on_r, writes_blank_on_l):
    /// - writes_blank_on_r is true if any transition writes 0 and moves Right.
    /// - writes_blank_on_l is true if any transition writes 0 and moves Left.
    ///
    /// This enables one-sided "fresh blank" invariants: if blank is never
    /// written on R-moves, then any 0 to the left of the head must be
    /// unvisited; similarly for the right side with L-moves.
    fn blank_writes_by_shift(&self) -> (bool, bool) {
        let mut on_r = false;
        let mut on_l = false;

        for (_, &(print, shift, _)) in self.iter() {
            if print != 0 {
                continue;
            }
            if shift {
                on_r = true;
            } else {
                on_l = true;
            }

            if on_r && on_l {
                break;
            }
        }

        (on_r, on_l)
    }

    /// Compute a sound over-approximation of the parity of the number of
    /// nonblank tape cells in each state, starting from the blank tape.
    ///
    /// A transition changes this parity exactly when one of `read` and
    /// `print` is blank and the other is nonblank. We retain only the state
    /// and this one parity bit, so every concrete run maps to a path in this
    /// finite abstract graph. Consequently, any absent parity bit is a valid
    /// invariant for backward pruning.
    fn nonblank_parity_from_blank(&self) -> NonblankParity<s> {
        let mut possible = [0_u8; s];
        possible[0] = 0b01; // initial state, entirely blank tape

        loop {
            let mut changed = false;

            for ((state, read), &(print, _, next_state)) in self.iter()
            {
                let state = state as usize;
                let next_state = next_state as usize;

                if state >= s || next_state >= s {
                    continue;
                }

                let source = possible[state];
                if source == 0 {
                    continue;
                }

                let flips = (read == 0) != (print == 0);
                let reached = if flips {
                    ((source & 0b01) << 1) | ((source & 0b10) >> 1)
                } else {
                    source
                };

                let old = possible[next_state];
                possible[next_state] |= reached;
                changed |= possible[next_state] != old;
            }

            if !changed {
                break;
            }
        }

        NonblankParity { possible }
    }

    /// Compute a *sound* shift-side restriction for each color.
    ///
    /// For a non-blank color `k != 0`:
    /// - If the machine never writes `k` on an L-move, then `k` can never
    ///   appear to the **right** of the head in any run from the
    ///   blank tape.
    /// - If the machine never writes `k` on an R-move, then `k` can
    ///   never appear to the **left** of the head in any run from the
    ///   blank tape.
    ///
    /// This is the classic invariant used in "shift-side" analysis:
    /// to get a symbol to the opposite side of the head you must
    /// *cross* it, and crossing requires leaving it behind via a move
    /// in that direction. If that direction never writes the symbol,
    /// the symbol cannot survive the crossing.
    fn shift_side_forbidden(&self) -> ([bool; c], [bool; c]) {
        // right_writes[k] == true if *any* transition writes k and moves R
        // left_writes[k]  == true if *any* transition writes k and moves L
        let mut left_writes = [false; c];
        let mut right_writes = [false; c];

        for (_, &(print, shift, _)) in self.iter() {
            (if shift {
                &mut right_writes
            } else {
                &mut left_writes
            })[print as usize] = true;
        }

        let mut forbid_left = [false; c];
        let mut forbid_right = [false; c];

        // Never forbid blanks (0) on either side.
        for k in 1..c {
            // If k is never written on an R-move, it cannot appear on the left.
            forbid_left[k] = !right_writes[k];
            // If k is never written on an L-move, it cannot appear on the right.
            forbid_right[k] = !left_writes[k];
        }

        (forbid_left, forbid_right)
    }

    /// Compute a sound over-approximation of which *immediate neighbor
    /// colors* can appear next to the head in each (state, scanned
    /// color), starting from the blank tape.
    ///
    /// We explore the abstract state space (q, L, S, R) where L and R
    /// are the colors immediately to the left/right of the head, and S
    /// is the scanned color. When the head moves off the 3-cell
    /// window, a known-blank outside tail exposes an exact zero;
    /// otherwise we conservatively treat the newly exposed cell as
    /// *unknown* (any color 0..c-1). This makes the analysis an
    /// over-approximation, and therefore safe for pruning: if a
    /// neighbor color is *not* possible here, it is not possible in any
    /// concrete run from blank.
    #[expect(clippy::cast_possible_truncation)]
    fn win_possible_from_blank(
        &self,
        forbid_left: &[bool; c],
        forbid_right: &[bool; c],
    ) -> WinPossible<s, c> {
        // Abstract state: (st, lb, l, sc, r, rb).
        // lb/rb = whether the whole tail immediately outside the 3-cell
        // window on that side is known blank.  The cells need not be
        // unvisited: only their current colors matter to this abstraction.
        fn idx<const C: usize, const S: usize>(
            st: usize,
            lb: usize,
            l: usize,
            sc: usize,
            r: usize,
            rb: usize,
        ) -> usize {
            // st * 2 * C^3 * 2 + ...
            let mut x = st;
            x = x * 2 + lb;
            x = x * C + l;
            x = x * C + sc;
            x = x * C + r;
            x = x * 2 + rb;
            x
        }

        let total = s * 2 * c * c * c * 2;
        let mut visited = vec![false; total];
        let mut q = std::collections::VecDeque::new();

        // Start from true blank: window 0 0 0 and both outsides known blank.
        q.push_back((0, 1, 0, 0, 0, 1));
        visited[idx::<c, s>(0, 1, 0, 0, 0, 1)] = true;

        assert!(c <= 64, "window bitmasks support at most 64 colors");

        let mut possible = WinPossible {
            right: [[[0; c]; c]; s],
            left: [[[0; c]; c]; s],
            any: [[false; c]; s],
        };

        while let Some((st, lb, l, sc, r, rb)) = q.pop_front() {
            possible.right[st][sc][l] |= 1_u64 << r;
            possible.left[st][sc][r] |= 1_u64 << l;
            possible.any[st][sc] = true;

            let st_state = st as State;
            let sc_color = sc as Color;

            let Some(&(print, shift, next_state)) =
                self.get(&(st_state, sc_color))
            else {
                // Missing transition: halting sink.
                continue;
            };

            let p = print as usize;
            let ns = next_state as usize;

            if shift {
                // Move Right.
                // New: left neighbor becomes printed symbol p, scanned becomes old r.
                // The new left tail starts at old l, so it remains known blank exactly
                // when old l is blank and the old farther-left tail was known blank.
                let new_lb = usize::from(lb == 1 && l == 0);

                if rb == 1 {
                    // The old right tail starts at the newly exposed cell, so both
                    // that cell and everything beyond it are known blank.  This does
                    // not depend on the old right neighbor r.
                    let n = (ns, new_lb, p, r, 0, 1);
                    let id = idx::<c, s>(n.0, n.1, n.2, n.3, n.4, n.5);
                    if !visited[id] {
                        visited[id] = true;
                        q.push_back(n);
                    }
                } else {
                    // The newly exposed cell is unknown; conservatively allow any
                    // right-side color and drop right-tail certainty.
                    for new_r in 0..c {
                        if forbid_right[new_r] {
                            continue;
                        }
                        let n = (ns, new_lb, p, r, new_r, 0);
                        let id =
                            idx::<c, s>(n.0, n.1, n.2, n.3, n.4, n.5);
                        if !visited[id] {
                            visited[id] = true;
                            q.push_back(n);
                        }
                    }
                }
            } else {
                // Move Left.  Symmetrically, the new right tail starts at old r.
                let new_rb = usize::from(rb == 1 && r == 0);

                if lb == 1 {
                    // The old left tail starts at the newly exposed cell, so that
                    // cell and everything beyond it are known blank.
                    let n = (ns, 1, 0, l, p, new_rb);
                    let id = idx::<c, s>(n.0, n.1, n.2, n.3, n.4, n.5);
                    if !visited[id] {
                        visited[id] = true;
                        q.push_back(n);
                    }
                } else {
                    // The newly exposed cell is unknown; conservatively allow any
                    // left-side color and drop left-tail certainty.
                    for new_l in 0..c {
                        if forbid_left[new_l] {
                            continue;
                        }
                        let n = (ns, 0, new_l, l, p, new_rb);
                        let id =
                            idx::<c, s>(n.0, n.1, n.2, n.3, n.4, n.5);
                        if !visited[id] {
                            visited[id] = true;
                            q.push_back(n);
                        }
                    }
                }
            }
        }

        possible
    }

    /// Compute a sound state-aware over-approximation of all colors and
    /// adjacent pairs that may occur anywhere on each side of the head.
    ///
    /// The fixed point is seeded by the two infinite blank sides.  On an
    /// R-move the printed color is prepended to the left side, creating the
    /// pair `(print, old-left-neighbor)`; the right side becomes a suffix of
    /// its former value, so propagating its old summary is conservative.
    /// L-moves are symmetric.  Summaries are joined by state, deliberately
    /// forgetting correlations with the scanned color.
    fn side_possible_from_blank(
        &self,
        win_possible: &WinPossible<s, c>,
    ) -> SidePossible<s, c> {
        assert!(c <= 64, "side bitmasks support at most 64 colors");

        let mut possible = SidePossible {
            colors: [[0; 2]; s],
            pairs: [[[0; c]; 2]; s],
        };

        if s == 0 || c == 0 {
            return possible;
        }

        for side in [LEFT_SIDE, RIGHT_SIDE] {
            possible.colors[0][side] = 1;
            possible.pairs[0][side][0] = 1;
        }

        loop {
            let mut changed = false;

            for ((state, read), &(print, shift, next_state)) in
                self.iter()
            {
                let st = state as usize;
                let sc = read as usize;
                let pr = print as usize;
                let ns = next_state as usize;

                if st >= s
                    || sc >= c
                    || pr >= c
                    || ns >= s
                    || !win_possible.any[st][sc]
                {
                    continue;
                }

                // Moving onto a side removes its nearest cell, while moving
                // away from the other side prepends one cell.  In either case
                // all colors/pairs surviving afterwards already occurred in
                // the source side, so copying both source summaries is safe.
                for side in [LEFT_SIDE, RIGHT_SIDE] {
                    let source_colors = possible.colors[st][side];
                    let old_colors = possible.colors[ns][side];
                    possible.colors[ns][side] =
                        old_colors | source_colors;
                    changed |= possible.colors[ns][side] != old_colors;

                    for near in 0..c {
                        let source_pairs =
                            possible.pairs[st][side][near];
                        let old_pairs = possible.pairs[ns][side][near];
                        possible.pairs[ns][side][near] =
                            old_pairs | source_pairs;
                        changed |=
                            possible.pairs[ns][side][near] != old_pairs;
                    }
                }

                #[expect(clippy::branches_sharing_code)]
                let (push_side, neighbor_mask) = if shift {
                    // R: printed cell becomes the new immediate left neighbor.
                    let mut mask = 0;
                    for right in 0..c {
                        mask |= win_possible.left[st][sc][right];
                    }
                    (LEFT_SIDE, mask)
                } else {
                    // L: printed cell becomes the new immediate right neighbor.
                    let mut mask = 0;
                    for left in 0..c {
                        mask |= win_possible.right[st][sc][left];
                    }
                    (RIGHT_SIDE, mask)
                };

                let old_colors = possible.colors[ns][push_side];
                possible.colors[ns][push_side] |= 1_u64 << pr;
                changed |= possible.colors[ns][push_side] != old_colors;

                let old_pairs = possible.pairs[ns][push_side][pr];
                possible.pairs[ns][push_side][pr] |= neighbor_mask;
                changed |=
                    possible.pairs[ns][push_side][pr] != old_pairs;
            }

            if !changed {
                break;
            }
        }

        possible
    }
}

#[cfg(test)]
use crate::instrs::{read_color, read_shift, read_state};

#[cfg(test)]
fn read_entry(entry: &str) -> Entry {
    let (slot, instr) = entry.split_once(':').unwrap();

    let mut chars = instr.chars();
    let color = chars.next().unwrap();
    let shift = chars.next().unwrap();

    (Slot::read(slot), (read_color(color), read_shift(shift)))
}

#[cfg(test)]
macro_rules! assert_entrypoints {
    ($(($prog:literal, ($s:literal, $c:literal)) => [$($state:literal => ($same:expr, $diff:expr)),* $(,)?]),* $(,)?) => {
        $({
            let mut entrypoints = Entrypoints::new();

            $(
                entrypoints.insert(
                    read_state($state),
                    (
                        $same.into_iter().map(read_entry).collect(),
                        $diff.into_iter().map(read_entry).collect(),
                    ),
                );
            )*

            assert_eq!(
                entrypoints,
                Prog::<$s, $c>::from($prog).get_entrypoints(),
            );
        })*
    };
}

#[test]
fn test_entrypoints() {
    assert_entrypoints!(
        ("1RB ...  1LB 0RB", (2, 2)) => [
            'B' => (["B0:1L", "B1:0R"], ["A0:1RB"])
        ],
        ("1RB ... ...  0LB 2RB 0RB", (2, 3)) => [
            'B' => (["B0:0L", "B1:2R", "B2:0R"], ["A0:1RB"])
        ],
        ("1RB ... 2LB  2LB 2RA 0RA", (2, 3)) => [
            'A' => ([], ["B1:2R", "B2:0R"]),
            'B' => (["B0:2L"], ["A0:1R", "A2:2L"])
        ],
        ("1RB 0RB 1RA  1LB 2RB 0LA", (2, 3)) => [
            'A' => (["A2:1R"], ["B2:0L"]),
            'B' => (["B0:1L", "B1:2R"], ["A0:1R", "A1:0R"])
        ],
        ("1RB 1RC  0LA 1RA  0LB ...", (3, 2)) => [
            'A' => ([], ["B0:0L", "B1:1R"]),
            'B' => ([], ["A0:1R", "C0:0L"]),
            'C' => ([], ["A1:1R"])
        ],
        ("1RB ...  0LB 1RC  0LC 1RA", (3, 2)) => [
            'A' => ([], ["C1:1R"]),
            'B' => (["B0:0L"], ["A0:1R"]),
            'C' => (["C0:0L"], ["B1:1R"])
        ],
        ("1RB 1LB  1LA 1LC  1RC 0LC", (3, 2)) => [
            'A' => ([], ["B0:1L"]),
            'B' => ([], ["A0:1R", "A1:1L"]),
            'C' => (["C0:1R", "C1:0L"], ["B1:1L"])
        ],
        ("1RB 0LC  1LB 1LA  1RC 0LC", (3, 2)) => [
            'A' => ([], ["B1:1L"]),
            'B' => (["B0:1L"], ["A0:1R"]),
            'C' => (["C0:1R", "C1:0L"], ["A1:0L"])
        ],
        ("1RB 2RA 0RB 2RB  1LB 3RB 3LA 0LA", (2, 4)) => [
            'A' => (["A1:2R"], ["B2:3L", "B3:0L"]),
            'B' => (["B0:1L", "B1:3R"], ["A0:1R", "A2:0R", "A3:2R"])
        ],
        ("1RB ...  0LC ...  1RC 1LD  0LC 0LD", (4, 2)) => [
            'B' => ([], ["A0:1RB"]),
            'C' => (["C0:1R"], ["B0:0L", "D0:0L"]),
            'D' => (["D1:0L"], ["C1:1L"])
        ],
        ("1RB ...  0LC ...  1RC 1LD  0LC 0LB", (4, 2)) => [
            'B' => ([], ["A0:1RB", "D1:0L"]),
            'C' => (["C0:1R"], ["B0:0L", "D0:0L"]),
            'D' => ([], ["C1:1L"])
        ],
        ("1RB 1LC  1RD 1RB  0RD 0RC  1LD 1LA", (4, 2)) => [
            'A' => ([], ["D1:1L"]),
            'B' => (["B1:1R"], ["A0:1R"]),
            'C' => (["C1:0R"], ["A1:1L"]),
            'D' => (["D0:1L"], ["B0:1R", "C0:0R"])
        ],
        ("1RB 1LC  0LC 0RD  1RD 1LE  1RE 1LA  1LA 0LB", (5, 2)) => [
            'A' => ([], ["D1:1L", "E0:1L"]),
            'B' => ([], ["A0:1R", "E1:0L"]),
            'C' => ([], ["A1:1L", "B0:0L"]),
            'D' => ([], ["B1:0R", "C0:1R"]),
            'E' => ([], ["C1:1L", "D0:1R"])
        ],
    );
}

/**************************************/

#[derive(Clone)]
struct Config {
    state: State,
    tape: Tape,
}

impl Config {
    const fn new(state: State, tape: Tape) -> Self {
        Self { state, tape }
    }

    const fn init_halt(state: State, color: Color) -> Self {
        Self::new(state, Tape::init_halt(color))
    }

    const fn init_blank(state: State, color: Color) -> Self {
        Self::new(state, Tape::init_blank(color))
    }

    const fn init_spinout(state: State, shift: Shift) -> Self {
        Self::new(state, Tape::init_spinout(shift))
    }

    fn init_twostep(state: State, l_co: Color, r_co: Color) -> Self {
        Self::new(state, Tape::init_twostep(l_co, r_co))
    }
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let tape = &self.tape;
        let slot = (self.state, tape.scan).show();

        write!(f, "{slot} | {tape}")
    }
}

/**************************************/

#[derive(Clone, PartialEq, Eq, Hash)]
enum TapeEnd {
    Blanks,
    Unknown,
}

type Count = u8;

/// A run count used only by the backward prover.
///
/// `Exact(n)` denotes exactly `n` cells. `AtLeast(n)` denotes an arbitrary
/// finite run of at least `n` cells.  Keeping the lower bound allows a run
/// created as `c..` to become `c^2..`, `c^3..`, ... as definite cells are
/// prepended, rather than losing that information forever.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum BlockCount {
    Exact(Count),
    AtLeast(Count),
}

impl BlockCount {
    const fn exact(count: Count) -> Self {
        debug_assert!(count > 0);
        Self::Exact(count)
    }

    const fn at_least(count: Count) -> Self {
        debug_assert!(count > 0);
        Self::AtLeast(count)
    }

    const fn minimum(self) -> Count {
        match self {
            Self::Exact(count) | Self::AtLeast(count) => count,
        }
    }

    const fn is_exact(self) -> bool {
        matches!(self, Self::Exact(_))
    }

    const fn is_single(self) -> bool {
        matches!(self, Self::Exact(1))
    }

    const fn is_indef(self) -> bool {
        matches!(self, Self::AtLeast(_))
    }

    const fn can_be_one(self) -> bool {
        matches!(self, Self::AtLeast(1))
    }

    const fn add_exact(&mut self, add: Count) {
        debug_assert!(add > 0);
        *self = match *self {
            Self::Exact(count) => Self::Exact(count + add),
            Self::AtLeast(count) => Self::AtLeast(count + add),
        };
    }

    const fn add_at_least(&mut self, add: Count) {
        debug_assert!(add > 0);
        *self = Self::AtLeast(self.minimum() + add);
    }

    /// Remove one definitely present cell.
    ///
    /// `AtLeast(1)` is used only on the residual branch where the concrete
    /// run had length at least two, so its residual is again `AtLeast(1)`.
    const fn decrement_after_pull(&mut self) {
        *self = match *self {
            Self::Exact(count) => {
                debug_assert!(count > 1);
                Self::Exact(count - 1)
            },
            Self::AtLeast(1) => Self::AtLeast(1),
            Self::AtLeast(count) => Self::AtLeast(count - 1),
        };
    }

    const fn subsumes(self, other: Self) -> bool {
        match (self, other) {
            (Self::Exact(a), Self::Exact(b)) => a == b,
            (Self::Exact(_), Self::AtLeast(_)) => false,
            (Self::AtLeast(a), Self::Exact(b) | Self::AtLeast(b)) => {
                a <= b
            },
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct Block {
    color: Color,
    count: BlockCount,
}

impl Block {
    const fn exact(color: Color, count: Count) -> Self {
        Self {
            color,
            count: BlockCount::exact(count),
        }
    }

    const fn at_least(color: Color, count: Count) -> Self {
        Self {
            color,
            count: BlockCount::at_least(count),
        }
    }

    const fn blank(&self) -> bool {
        self.color == 0
    }
}

impl fmt::Display for Block {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.count {
            BlockCount::Exact(1) => write!(f, "{}", self.color),
            BlockCount::Exact(count) => {
                write!(f, "{}^{count}", self.color)
            },
            BlockCount::AtLeast(1) => write!(f, "{}..", self.color),
            BlockCount::AtLeast(count) => {
                write!(f, "{}^{count}..", self.color)
            },
        }
    }
}

/// Minimal near-head span implementation for BKW.  Storage matches the shared
/// tape span: farthest block first, nearest block last.
#[derive(Clone, PartialEq, Eq, Hash)]
struct SpanT {
    blocks: Vec<Block>,
}

impl SpanT {
    const fn init_blank() -> Self {
        Self { blocks: vec![] }
    }

    const fn len(&self) -> usize {
        self.blocks.len()
    }

    const fn blank(&self) -> bool {
        self.blocks.is_empty()
    }

    fn iter(&self) -> impl DoubleEndedIterator<Item = &Block> {
        self.blocks.iter().rev()
    }

    fn str_iter(&self) -> impl DoubleEndedIterator<Item = String> + '_ {
        self.iter().map(ToString::to_string)
    }

    fn first(&self) -> Option<&Block> {
        self.blocks.last()
    }

    fn first_mut(&mut self) -> Option<&mut Block> {
        self.blocks.last_mut()
    }

    fn pop_block(&mut self) -> Block {
        self.blocks.pop().unwrap()
    }

    fn push_exact(&mut self, color: Color, count: Count) {
        if let Some(block) = self.first_mut()
            && block.color == color
        {
            block.count.add_exact(count);
            return;
        }

        self.blocks.push(Block::exact(color, count));
    }

    fn push_at_least(&mut self, color: Color, count: Count) {
        if let Some(block) = self.first_mut()
            && block.color == color
        {
            block.count.add_at_least(count);
            return;
        }

        self.blocks.push(Block::at_least(color, count));
    }

    fn push_block(&mut self, block: &Block) {
        match block.count {
            BlockCount::Exact(count) => {
                self.push_exact(block.color, count);
            },
            BlockCount::AtLeast(count) => {
                self.push_at_least(block.color, count);
            },
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct Span {
    span: SpanT,
    end: TapeEnd,
}

impl Span {
    const fn init_blank() -> Self {
        Self {
            span: SpanT::init_blank(),
            end: TapeEnd::Blanks,
        }
    }

    const fn init_unknown() -> Self {
        Self {
            span: SpanT::init_blank(),
            end: TapeEnd::Unknown,
        }
    }

    fn init_unknown_with(color: Color) -> Self {
        let mut span = Self {
            span: SpanT::init_blank(),
            end: TapeEnd::Unknown,
        };

        span.push_single(color);

        span
    }

    const fn end_str(&self) -> &str {
        match self.end {
            TapeEnd::Blanks => "0+",
            TapeEnd::Unknown => "?",
        }
    }

    fn blank(&self) -> bool {
        self.span.iter().all(Block::blank)
    }

    fn matches_color(&self, print: Color) -> bool {
        self.span.first().map_or_else(
            || match self.end {
                TapeEnd::Blanks => print == 0,
                TapeEnd::Unknown => true,
            },
            |block| block.color == print,
        )
    }

    fn pull(&mut self) {
        let Some(block) = self.span.first_mut() else {
            return;
        };

        if block.count.is_single() {
            self.span.pop_block();
        } else {
            block.count.decrement_after_pull();
        }
    }

    fn push_single(&mut self, color: Color) {
        if self.span.first().is_none()
            && color == 0
            && self.end == TapeEnd::Blanks
        {
            return;
        }

        self.span.push_exact(color, 1);
    }

    fn push_indef(&mut self, color: Color) {
        if color == 0
            && self.span.blank()
            && self.end == TapeEnd::Blanks
        {
            return;
        }

        self.span.push_at_least(color, 1);
    }

    fn set_head_to_one(&mut self) {
        let block = self.span.first_mut().unwrap();
        debug_assert!(block.count.can_be_one());
        block.count = BlockCount::Exact(1);
    }

    /// If this span's end is known to be all blanks (`0+`), then any explicit
    /// trailing blank blocks at the *far* end are redundant and can be dropped.
    ///
    /// This keeps canonical forms like `0+ 0 [x] ?` from persisting as distinct
    /// configurations; it becomes `0+ [x] ?`.
    fn absorb_trailing_blanks(&mut self) {
        if self.end != TapeEnd::Blanks {
            return;
        }

        // Collect blocks (ordered near->far) and drop blanks from the far end.
        let mut blocks: Vec<Block> =
            self.span.iter().cloned().collect();
        while matches!(blocks.last(), Some(b) if b.color == 0) {
            blocks.pop();
        }

        if blocks.len() == self.span.len() {
            return;
        }

        // Rebuild span by pushing blocks from far->near (push_block is near-end).
        let mut new_span = SpanT::init_blank();
        for b in blocks.into_iter().rev() {
            new_span.push_block(&b);
        }
        self.span = new_span;
    }
}

/**************************************/

#[derive(Clone, PartialEq, Eq)]
struct Tape {
    scan: Color,
    lspan: Span,
    rspan: Span,
    head: Pos,
}

impl Scan for Tape {
    fn scan(&self) -> Color {
        self.scan
    }
}

impl fmt::Display for Tape {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} {} {}",
            self.lspan.end_str(),
            self.lspan
                .span
                .str_iter()
                .rev()
                .chain(once(format!("[{}]", self.scan)))
                .chain(self.rspan.span.str_iter())
                .collect::<Vec<_>>()
                .join(" "),
            self.rspan.end_str(),
        )
    }
}

impl Tape {
    const fn init_halt(scan: Color) -> Self {
        Self {
            scan,
            lspan: Span::init_unknown(),
            rspan: Span::init_unknown(),
            head: 0,
        }
    }

    const fn init_blank(scan: Color) -> Self {
        Self {
            scan,
            lspan: Span::init_blank(),
            rspan: Span::init_blank(),
            head: 0,
        }
    }

    const fn init_spinout(dir: Shift) -> Self {
        if dir {
            Self::init_r_spinout()
        } else {
            Self::init_l_spinout()
        }
    }

    const fn init_r_spinout() -> Self {
        Self {
            scan: 0,
            lspan: Span::init_unknown(),
            rspan: Span::init_blank(),
            head: 0,
        }
    }

    const fn init_l_spinout() -> Self {
        Self {
            scan: 0,
            lspan: Span::init_blank(),
            rspan: Span::init_unknown(),
            head: 0,
        }
    }

    fn init_twostep(l_co: Color, r_co: Color) -> Self {
        Self {
            scan: l_co,
            lspan: Span::init_unknown(),
            rspan: Span::init_unknown_with(r_co),
            head: 0,
        }
    }

    fn blank(&self) -> bool {
        self.scan == 0 && self.lspan.blank() && self.rspan.blank()
    }

    /// Return the possible parities of the total number of nonblank cells.
    /// Bit 0 means even is possible; bit 1 means odd is possible.
    ///
    /// A fully bounded tape with fixed run counts has an exact parity. An
    /// unknown end or an indefinite nonblank run can realize either parity,
    /// so returning `0b11` is the sound conservative answer.
    fn nonblank_parity_mask(&self) -> u8 {
        let mut parity = u8::from(self.scan != 0);

        for span in [&self.lspan, &self.rspan] {
            if span.end == TapeEnd::Unknown {
                return 0b11;
            }

            for block in span.span.iter() {
                if block.color == 0 {
                    continue;
                }
                if block.count.is_indef() {
                    return 0b11;
                }

                parity ^= u8::from(block.count.minimum() & 1 != 0);
            }
        }

        1_u8 << parity
    }

    fn hash(&self) -> u64 {
        let mut h = AHasher::default();
        self.scan.hash(&mut h);
        self.lspan.hash(&mut h);
        self.rspan.hash(&mut h);
        h.finish()
    }

    /// Return the immediate left neighbor color if it is determined by
    /// this tape description. If the left side is completely unknown
    /// (`?`) and there are no explicit blocks, returns None.
    fn left_neighbor_color(&self) -> Option<Color> {
        self.lspan.span.first().map(|b| b.color).or_else(|| {
            matches!(self.lspan.end, TapeEnd::Blanks).then_some(0)
        })
    }

    /// Return the immediate right neighbor color if it is determined by
    /// this tape description. If the right side is completely unknown
    /// (`?`) and there are no explicit blocks, returns None.
    fn right_neighbor_color(&self) -> Option<Color> {
        self.rspan.span.first().map(|b| b.color).or_else(|| {
            matches!(self.rspan.end, TapeEnd::Blanks).then_some(0)
        })
    }

    fn is_valid_step(&self, shift: Shift, print: Color) -> bool {
        (if shift { &self.lspan } else { &self.rspan })
            .matches_color(print)
    }

    const fn is_spinout(&self, shift: Shift, read: Color) -> bool {
        if self.scan != read {
            return false;
        }

        let pull = if shift { &self.lspan } else { &self.rspan };

        pull.span.blank()
    }

    /// `AtLeast(1)` needs two predecessor branches when pulled: concrete
    /// length one disappears, while concrete length at least two leaves an
    /// `AtLeast(1)` residual. Larger lower bounds decrement without a split.
    fn pull_needs_count_one_split(&self, shift: Shift) -> bool {
        let pull = if shift { &self.lspan } else { &self.rspan };

        let Some(block) = pull.span.first() else {
            return false;
        };

        block.count.can_be_one()
    }

    fn backstep(&mut self, shift: Shift, read: Color) {
        let (stepped, pull, push) = if shift {
            (-1, &mut self.lspan, &mut self.rspan)
        } else {
            (1, &mut self.rspan, &mut self.lspan)
        };

        pull.pull();

        push.push_single(self.scan);

        self.scan = read;

        self.head += stepped;
    }

    fn push_indef(&mut self, shift: Shift) {
        let push = if shift {
            &mut self.rspan
        } else {
            &mut self.lspan
        };

        push.push_indef(self.scan);
    }

    /// One-sided "fresh blank" invariants.
    ///
    /// Starting from the blank tape and moving one cell at a time, visited
    /// cells form a contiguous interval.
    ///
    /// - If the program never writes blank (`0`) on an R-move, then a cell
    ///   to the **left** of the head cannot end up as `0` via being visited
    ///   (because the last visit would have to leave it behind on a Right
    ///   move). So any observed `0` on the left must be unvisited, and thus
    ///   nothing non-blank can appear farther left.
    /// - Symmetrically, if the program never writes `0` on an L-move, any
    ///   observed `0` on the right must be unvisited, so nothing non-blank
    ///   can appear farther right.
    ///
    /// This is a *sound* pruning/normalization step that rejects impossible
    /// spans and can tighten `?` ends to `0+` when an explicit `0` block is
    /// present on the applicable side.
    fn enforce_fresh_zero_side_invariants(
        &mut self,
        left_fresh_zero: bool,
        right_fresh_zero: bool,
    ) -> bool {
        fn check_side(span: &mut Span) -> bool {
            let mut seen_zero = false;

            for b in span.span.iter() {
                if seen_zero && b.color != 0 {
                    return false; // nonblank beyond an unvisited blank
                }
                if b.color == 0 {
                    seen_zero = true;
                }
            }

            if seen_zero {
                // Beyond the outermost explicit cell is certainly blank.
                span.end = TapeEnd::Blanks;
                span.absorb_trailing_blanks();
            }

            true
        }

        let sides_ok = (if left_fresh_zero {
            check_side(&mut self.lspan)
        } else {
            true
        }) && (if right_fresh_zero {
            check_side(&mut self.rspan)
        } else {
            true
        });

        if !sides_ok {
            return false;
        }

        // If blank is never written in either direction, a scanned blank is
        // being visited for the first time. The previously visited interval
        // must therefore lie wholly on one side of the head. If an explicit
        // nonblank cell identifies that side, the opposite tail is forced to
        // be blank. Explicit nonblank cells on both sides are impossible.
        if self.scan == 0 && left_fresh_zero && right_fresh_zero {
            let left_nonblank =
                self.lspan.span.iter().any(|block| block.color != 0);
            let right_nonblank =
                self.rspan.span.iter().any(|block| block.color != 0);

            match (left_nonblank, right_nonblank) {
                (true, true) => return false,
                (true, false) => self.rspan = Span::init_blank(),
                (false, true) => self.lspan = Span::init_blank(),
                (false, false) => {},
            }
        }

        true
    }

    /// Reject explicit side colors forbidden by shift-side analysis.
    ///
    /// The three-cell window filter sees only immediate neighbors.  This
    /// check carries the same per-color invariant across every explicit block
    /// in both spans, so impossible colors cannot survive farther from the
    /// head.
    fn obeys_shift_side<const C: usize>(
        &self,
        forbid_left: &[bool; C],
        forbid_right: &[bool; C],
    ) -> bool {
        self.lspan
            .span
            .iter()
            .all(|block| !forbid_left[block.color as usize])
            && self
                .rspan
                .span
                .iter()
                .all(|block| !forbid_right[block.color as usize])
    }

    /// Check every explicit side color and adjacent pair against the
    /// state-aware forward fixed point.  Span blocks are stored nearest-head
    /// first, matching the pair orientation used by `SidePossible`.
    fn obeys_state_side<const S: usize, const C: usize>(
        &self,
        state: State,
        possible: &SidePossible<S, C>,
    ) -> bool {
        fn check_span<const C: usize>(
            span: &Span,
            color_mask: u64,
            pair_masks: &[u64; C],
        ) -> bool {
            let pair_possible = |near: Color, far: Color| {
                let near = near as usize;
                let far = far as usize;
                near < C
                    && far < C
                    && (pair_masks[near] & (1_u64 << far)) != 0
            };

            let mut previous = None;

            for block in span.span.iter() {
                let color = block.color as usize;
                if color >= C || (color_mask & (1_u64 << color)) == 0 {
                    return false;
                }

                if block.count.minimum() > 1
                    && !pair_possible(block.color, block.color)
                {
                    return false;
                }

                if let Some(near) = previous
                    && !pair_possible(near, block.color)
                {
                    return false;
                }

                previous = Some(block.color);
            }

            match (span.end.clone(), previous) {
                (TapeEnd::Blanks, Some(near)) => pair_possible(near, 0),
                (TapeEnd::Blanks, None) => pair_possible(0, 0),
                (TapeEnd::Unknown, Some(near)) => {
                    pair_masks[near as usize] != 0
                },
                (TapeEnd::Unknown, None) => true,
            }
        }

        let st = state as usize;
        st < S
            && check_span(
                &self.lspan,
                possible.colors[st][LEFT_SIDE],
                &possible.pairs[st][LEFT_SIDE],
            )
            && check_span(
                &self.rspan,
                possible.colors[st][RIGHT_SIDE],
                &possible.pairs[st][RIGHT_SIDE],
            )
    }

    /// Enforce sides proved to contain blanks only.
    ///
    /// Merely changing an unknown end to `0+` is insufficient when an
    /// explicit nonblank block is already present.  Such a tape contradicts
    /// the invariant and must be rejected.  Otherwise every explicit zero is
    /// redundant and the whole side can be canonicalized to a blank span.
    fn tighten_forced_blank_ends(
        &mut self,
        left_forced_blank: bool,
        right_forced_blank: bool,
    ) -> bool {
        fn force_blank(span: &mut Span) -> bool {
            if span.span.iter().any(|block| block.color != 0) {
                return false;
            }

            *span = Span::init_blank();
            true
        }

        (!left_forced_blank || force_blank(&mut self.lspan))
            && (!right_forced_blank || force_blank(&mut self.rspan))
    }

    fn subsumes(&self, other: &Self) -> bool {
        self.scan == other.scan
            && self.head == other.head
            && self.lspan.subsumes(&other.lspan)
            && self.rspan.subsumes(&other.rspan)
    }
}

/**************************************/

#[cfg(test)]
impl From<&str> for Block {
    fn from(s: &str) -> Self {
        if let Some(body) = s.strip_suffix("..") {
            if let Some((color, count)) = body.split_once('^') {
                return Self::at_least(
                    color.parse().unwrap(),
                    count.parse().unwrap(),
                );
            }

            return Self::at_least(body.parse().unwrap(), 1);
        }

        if let Some((color, count)) = s.split_once('^') {
            return Self::exact(
                color.parse().unwrap(),
                count.parse().unwrap(),
            );
        }

        Self::exact(s.parse().unwrap(), 1)
    }
}

#[cfg(test)]
impl Span {
    fn new(end: &str, blocks: Vec<Block>) -> Self {
        let mut span = (match end {
            "0+" => Self::init_blank,
            "?" => Self::init_unknown,
            _ => unreachable!(),
        })();

        for block in blocks {
            span.span.push_block(&block);
        }

        span
    }
}

#[cfg(test)]
impl From<&str> for Tape {
    fn from(s: &str) -> Self {
        let parts: Vec<&str> = s.split_whitespace().collect();

        let l_end = parts[0];

        assert!(matches!(l_end, "?" | "0+"));

        let l_blocks: Vec<Block> = parts[1..]
            .iter()
            .take_while(|p| !p.starts_with('['))
            .map(|&p| p.into())
            .collect::<Vec<_>>()
            .into_iter()
            .collect();

        let scan = parts
            .iter()
            .find(|p| p.starts_with('['))
            .and_then(|p| {
                p.trim_matches(|c| c == '[' || c == ']').parse().ok()
            })
            .unwrap();

        let rspan_start = parts
            .iter()
            .position(|&p| p.starts_with('['))
            .map_or(parts.len(), |pos| pos + 1);

        let r_end = *parts.last().unwrap();

        assert!(matches!(l_end, "?" | "0+"));

        let r_blocks: Vec<Block> = parts[rspan_start..parts.len() - 1]
            .iter()
            .map(|&p| p.into())
            .rev()
            .collect();

        Self {
            scan,
            head: 0,
            lspan: Span::new(l_end, l_blocks),
            rspan: Span::new(r_end, r_blocks),
        }
    }
}

/**************************************/

#[cfg(test)]
impl Tape {
    #[track_caller]
    fn assert(&self, exp: &str) {
        assert_eq!(self.to_string(), exp);
    }

    #[track_caller]
    fn tbackstep(
        &mut self,
        shift: u8,
        print: Color,
        read: Color,
        success: bool,
    ) {
        assert!(matches!(shift, 0 | 1));

        let shift = shift != 0;

        let step = self.is_valid_step(shift, print);

        assert_eq!(step, success);

        if !step {
            return;
        }

        self.backstep(shift, read);
    }
}

#[test]
fn test_backstep_halt() {
    let mut tape = Tape::init_halt(2);

    tape.assert("? [2] ?");

    tape.tbackstep(0, 2, 1, true);

    tape.assert("? 2 [1] ?");

    tape.tbackstep(1, 1, 2, false);

    tape.assert("? 2 [1] ?");

    tape.tbackstep(1, 2, 0, true);

    tape.assert("? [0] 1 ?");

    tape.tbackstep(1, 0, 2, true);

    tape.assert("? [2] 0 1 ?");
}

#[test]
fn test_backstep_blank() {
    let mut tape = Tape::init_blank(2);

    tape.assert("0+ [2] 0+");

    tape.tbackstep(0, 1, 1, false);
    tape.tbackstep(0, 2, 1, false);
    tape.tbackstep(0, 0, 1, true);

    tape.assert("0+ 2 [1] 0+");

    tape.tbackstep(1, 0, 0, false);
    tape.tbackstep(1, 1, 0, false);
    tape.tbackstep(1, 2, 0, true);

    tape.assert("0+ [0] 1 0+");

    tape.tbackstep(1, 1, 0, false);
    tape.tbackstep(1, 2, 0, false);
    tape.tbackstep(1, 0, 0, true);

    tape.assert("0+ [0] 0 1 0+");
}

#[test]
fn test_backstep_spinout() {
    let mut tape = Tape::init_spinout(true);

    tape.assert("? [0] 0+");

    tape.tbackstep(0, 1, 1, false);
    tape.tbackstep(0, 2, 1, false);
    tape.tbackstep(0, 0, 1, true);

    tape.assert("? 0 [1] 0+");

    tape.tbackstep(0, 1, 2, false);
    tape.tbackstep(0, 2, 2, false);
    tape.tbackstep(0, 0, 2, true);

    tape.assert("? 0 1 [2] 0+");

    tape.tbackstep(1, 1, 2, true);
    tape.tbackstep(1, 0, 1, true);
    tape.tbackstep(1, 0, 0, true);
    tape.tbackstep(1, 0, 0, true);

    tape.assert("? [0] 0 1 2^2 0+");
}

#[test]
fn test_backstep_required() {
    let mut tape: Tape = "0+ [1] 1 0 ?".into();

    tape.assert("0+ [1] 1 0 ?");

    tape.tbackstep(0, 1, 0, true);

    tape.assert("0+ 1 [0] 0 ?");
}

#[test]
fn test_spinout() {
    let mut tape: Tape = "0+ [1] 0^2 ?".into();

    tape.assert("0+ [1] 0^2 ?");

    assert!(!tape.is_valid_step(false, 1));
    assert!(tape.is_spinout(true, 1));

    tape.push_indef(true);

    tape.assert("0+ [1] 1.. 0^2 ?");

    assert!(!tape.is_spinout(false, 1));
    assert!(tape.is_spinout(true, 1));
}

#[test]
fn test_parse() {
    let tapes = [
        "? 2 1^2 [5] 3^3 0+",
        "0+ 2 1^2 [5] 3^3 ?",
        "0+ 2 1^2 [5] 3^3 0+",
        "? 2 3^11 4 1^11 [0] ?",
        "? 2 3^11 4 1^11 [0] 0+",
        "0+ 2 3^11 4 1^11 [0] ?",
        "? 4^118 [4] 5^2 2 4 5^7 1 0+",
        "? 4^118 [4] 5^2 2 4 5^7 1 0+",
        "0+ 4^118 [4] 5^2 2 4 5^7 1 0+",
    ];

    for tape in tapes {
        Into::<Tape>::into(tape).assert(tape);
    }
}

#[test]
fn test_backstep_indef() {
    let mut tape: Tape = "0+ [1] 1.. 0^2 ?".into();

    tape.backstep(false, 1);

    tape.assert("0+ 1 [1] 1.. 0^2 ?");
}

#[test]
fn test_push_indef() {
    let mut tape: Tape = "0+ 1 [0] ?".into();

    tape.push_indef(false);

    tape.assert("0+ 1 0.. [0] ?");

    tape.assert("0+ 1 0.. [0] ?");

    tape.scan = 1;
    tape.push_indef(false);

    tape.assert("0+ 1 0.. 1.. [1] ?");

    tape.scan = 0;
    tape.push_indef(false);

    tape.assert("0+ 1 0.. 1.. 0.. [0] ?");

    tape.backstep(false, 0);

    tape.assert("0+ 1 0.. 1.. 0^2.. [0] ?");
}

#[test]
fn test_lower_bounded_indefinite_runs() {
    let mut tape: Tape = "0+ [0] 1^3.. ?".into();

    assert!(!tape.pull_needs_count_one_split(false));
    tape.backstep(false, 0);
    tape.assert("0+ [0] 1^2.. ?");

    // Definite and indefinite same-color pushes both raise the lower bound.
    let mut pushed: Tape = "0+ 1^2.. [1] ?".into();
    pushed.backstep(false, 0);
    pushed.assert("0+ 1^3.. [0] ?");
    pushed.scan = 1;
    pushed.push_indef(false);
    pushed.assert("0+ 1^4.. [1] ?");

    let merged: Tape = "0+ [0] 1 1^2.. ?".into();
    merged.assert("0+ [0] 1^3.. ?");
    assert!(!merged.pull_needs_count_one_split(false));

    let mut split: Tape = "0+ [0] 1.. ?".into();
    assert!(split.pull_needs_count_one_split(false));
    split.rspan.set_head_to_one();
    split.assert("0+ [0] 1 ?");
}

#[test]
fn test_lower_bounded_subsumption() {
    let broad: Tape = "? [0] 1^3.. ?".into();
    let exact_large: Tape = "? [0] 1^5 ?".into();
    let narrower: Tape = "? [0] 1^4.. ?".into();
    let too_small: Tape = "? [0] 1^2 ?".into();
    let too_broad: Tape = "? [0] 1^2.. ?".into();

    assert!(broad.subsumes(&exact_large));
    assert!(broad.subsumes(&narrower));
    assert!(!broad.subsumes(&too_small));
    assert!(!broad.subsumes(&too_broad));
    assert!(!exact_large.subsumes(&broad));
}

#[test]
fn test_nonblank_parity_pruning() {
    let prog =
        Prog::<3, 3>::from("1RB 2RA 1LC  2LC 1RB 2RB  ... 2LA 1LA");
    let parity = prog.nonblank_parity_from_blank();

    // For this machine, A and C always have even support while B always has
    // odd support.
    assert_eq!(parity.possible, [0b01, 0b10, 0b01]);

    let even: Tape = "0+ 2 1 [0] 0+".into();
    assert_eq!(even.nonblank_parity_mask(), 0b01);
    assert!(nonblank_parity_possible(0, &even, &parity));
    assert!(!nonblank_parity_possible(1, &even, &parity));
    assert!(nonblank_parity_possible(2, &even, &parity));

    let odd: Tape = "0+ 2 [0] 0+".into();
    assert_eq!(odd.nonblank_parity_mask(), 0b10);
    assert!(!nonblank_parity_possible(0, &odd, &parity));
    assert!(nonblank_parity_possible(1, &odd, &parity));

    // An unresolved tail or an indefinite nonblank run remains conservative.
    let unknown: Tape = "0+ 2 1 [0] ?".into();
    assert_eq!(unknown.nonblank_parity_mask(), 0b11);
    assert!(nonblank_parity_possible(1, &unknown, &parity));

    let indefinite: Tape = "0+ 1.. [0] 0+".into();
    assert_eq!(indefinite.nonblank_parity_mask(), 0b11);
}

#[test]
fn test_scanned_fresh_zero_closes_opposite_tail() {
    let prog =
        Prog::<3, 3>::from("1RB 2RA 1LC  2LC 1RB 2RB  ... 2LA 1LA");
    let parity = prog.nonblank_parity_from_blank();

    // This is one of the backward B0 branches from the example. Since the
    // program never writes blank, the scanned 0 is fresh; the unknown tail to
    // its right is therefore all blank. Its support parity is then exactly
    // even, contradicting state B.
    let mut branch: Tape = "0+ 2 1 [0] ?".into();
    assert!(branch.enforce_fresh_zero_side_invariants(true, true));
    branch.assert("0+ 2 1 [0] 0+");
    assert!(!nonblank_parity_possible(1, &branch, &parity));

    // A fresh scanned blank cannot have previously visited/nonblank cells on
    // both sides.
    let mut impossible: Tape = "? 1 [0] 2 ?".into();
    assert!(!impossible.enforce_fresh_zero_side_invariants(true, true));
}

#[test]
fn test_state_side_colors_and_pairs() {
    let prog = Prog::<2, 2>::from("1RB ...  ... ...");
    let (forbid_left, forbid_right) = prog.shift_side_forbidden();
    let windows =
        prog.win_possible_from_blank(&forbid_left, &forbid_right);
    let sides = prog.side_possible_from_blank(&windows);

    // After the only step, state B has `1 0+` on the left and `0+` on the
    // right.  The whole-side abstraction retains the infinite (0,0) tail and
    // the newly created boundary pair (1,0).
    assert_eq!(sides.colors[1][LEFT_SIDE], 0b11);
    assert_eq!(sides.colors[1][RIGHT_SIDE], 0b01);
    assert_ne!(sides.pairs[1][LEFT_SIDE][1] & 0b01, 0);
    assert_eq!(sides.pairs[1][LEFT_SIDE][1] & 0b10, 0);

    let valid: Tape = "0+ 1 [0] 0+".into();
    assert!(valid.obeys_state_side(1, &sides));

    let impossible_right: Tape = "0+ [0] 1 0+".into();
    assert!(!impossible_right.obeys_state_side(1, &sides));

    // A fixed run of length two requires (1,1), which never occurs here.
    let impossible_pair: Tape = "0+ 1^2 [0] 0+".into();
    assert!(!impossible_pair.obeys_state_side(1, &sides));

    // An indefinite run denotes one-or-more cells, so it can still choose
    // length one and must not require the self-pair.
    let indefinite: Tape = "0+ 1.. [0] 0+".into();
    assert!(indefinite.obeys_state_side(1, &sides));

    // A lower bound of two guarantees an internal (1,1) adjacency.
    let at_least_two: Tape = "0+ 1^2.. [0] 0+".into();
    assert!(!at_least_two.obeys_state_side(1, &sides));
}

#[test]
#[expect(clippy::shadow_unrelated)]
fn test_halfblank_direction() {
    // A0 writes 1 and moves right.  At B0 the right side is still all blank,
    // but the left side contains the written 1.
    let prog = Prog::<2, 2>::from("1RB ...  ... ...");
    let (forbid_left, forbid_right) = prog.shift_side_forbidden();
    let windows =
        prog.win_possible_from_blank(&forbid_left, &forbid_right);
    let left_clean = side_excursions(&prog, &windows, false, true);
    let right_clean = side_excursions(&prog, &windows, true, true);
    let left_any = side_excursions(&prog, &windows, false, false);
    let right_any = side_excursions(&prog, &windows, true, false);
    let left = halfblank_slots(
        &prog,
        &windows,
        false,
        &left_clean,
        &right_any,
    );
    let right =
        halfblank_slots(&prog, &windows, true, &right_clean, &left_any);

    assert!(!left[1][0]);
    assert!(right[1][0]);

    // Leaving 0 behind preserves both one-sided-blank abstractions.
    let prog = Prog::<2, 2>::from("0RB ...  ... ...");
    let (forbid_left, forbid_right) = prog.shift_side_forbidden();
    let windows =
        prog.win_possible_from_blank(&forbid_left, &forbid_right);
    let left_clean = side_excursions(&prog, &windows, false, true);
    let right_clean = side_excursions(&prog, &windows, true, true);
    let left_any = side_excursions(&prog, &windows, false, false);
    let right_any = side_excursions(&prog, &windows, true, false);
    let left = halfblank_slots(
        &prog,
        &windows,
        false,
        &left_clean,
        &right_any,
    );
    let right =
        halfblank_slots(&prog, &windows, true, &right_clean, &left_any);

    assert!(left[1][0]);
    assert!(right[1][0]);
}

#[test]
fn test_parent_color_aware_excursions() {
    // A0 pushes right, writing 0 on the parent. B1 pops left into C, then C0
    // pops left across A's outer boundary into D.  The synthetic forward-window
    // abstraction permits child color 1 at A0 only when A0's own back/left
    // neighbor is 1, not when it is 0.  The old union-over-back-colors relation
    // would therefore invent an outer return for back color 0 as well.
    let prog = Prog::<4, 2>::from("0RB ...  ... 0LC  0LD ...  ... ...");
    let mut windows = WinPossible {
        right: [[[0; 2]; 2]; 4],
        left: [[[0; 2]; 2]; 4],
        any: [[false; 2]; 4],
    };

    // A0: with left/back 0, right child must be 0; with left/back 1, child 1.
    windows.right[0][0][0] = 1 << 0;
    windows.right[0][0][1] = 1 << 1;

    // B1 with parent/back 0 can pop back to A's cell in state C.
    windows.right[1][1][0] = 1 << 0;

    // C0 can then pop across the outer boundary for either outer back color.
    windows.right[2][0][0] = 1 << 0;
    windows.right[2][0][1] = 1 << 0;

    let ex = side_excursions(&prog, &windows, true, false);

    assert_eq!(ex.ret_states(0, 0, 0) & (1 << 3), 0);
    assert_ne!(ex.ret_states(1, 0, 0) & (1 << 3), 0);
}

#[test]
#[expect(clippy::shadow_unrelated, clippy::iter_on_single_items)]
fn test_halt_side_excursion_filter() {
    // Fresh zero at a right frontier: after A0 moves right, B0 is a valid
    // halting shape because the newly scanned cell is blank and the right
    // side is still globally blank.
    let prog = Prog::<2, 2>::from("1RB ...  ... ...");
    let slots: Set<Slot> = [(1, 0)].into_iter().collect();
    assert!(prog.halt_slots_side_excursion(slots).contains(&(1, 0)));

    // Nonzero halt after a genuine one-sided return.  A0 writes 1 and moves
    // right; B0 returns left into C, so C1 is a realizable halt slot.
    let prog = Prog::<3, 2>::from("1RB ...  0LC ...  ... ...");
    let slots: Set<Slot> = [(2, 1)].into_iter().collect();
    assert!(prog.halt_slots_side_excursion(slots).contains(&(2, 1)));

    // If the child side has no way to return across the parent boundary, the
    // same nonzero halt slot has no last-departure witness.
    let prog = Prog::<3, 2>::from("1RB ...  0RB ...  ... ...");
    #[expect(clippy::shadow_unrelated)]
    let slots: Set<Slot> = [(2, 1)].into_iter().collect();
    assert!(prog.halt_slots_side_excursion(slots).is_empty());
}

/**************************************/

use core::array::from_fn;
use std::{collections::VecDeque, sync::Arc};

type Adj<const S: usize> = [Vec<usize>; S];
type Preds<const S: usize> = [[Vec<usize>; 2]; S]; // preds[v][dir] -> u
type Writers<const C: usize> = [[Vec<usize>; 2]; C]; // writers[color][dir] -> v
type NextDir<const S: usize> = [[Vec<usize>; 2]; S]; // next[u][dir] -> v
type Indices<const S: usize, const C: usize> =
    (Adj<S>, Preds<S>, Writers<C>, NextDir<S>);

fn indices_new<const S: usize, const C: usize>() -> Indices<S, C> {
    (
        from_fn(|_| vec![]),
        from_fn(|_| from_fn(|_| vec![])),
        from_fn(|_| from_fn(|_| vec![])),
        from_fn(|_| from_fn(|_| vec![])),
    )
}

fn indices_add<const S: usize, const C: usize>(
    (adj, preds, writers, next): &mut Indices<S, C>,
    st: State,
    tr: State,
    sh: Shift,
    pr: Color,
) {
    let (st, tr, sh, pr) =
        (st as usize, tr as usize, usize::from(sh), pr as usize);

    adj[st].push(tr);
    preds[tr][sh].push(st);
    writers[pr][sh].push(tr);
    next[st][sh].push(tr);
}

fn indices_finalize<const S: usize, const C: usize>(
    (adj, preds, writers, next): &mut Indices<S, C>,
) {
    for u in 0..S {
        adj[u].sort_unstable();
        adj[u].dedup();
        for d in 0..2 {
            preds[u][d].sort_unstable();
            preds[u][d].dedup();
            next[u][d].sort_unstable();
            next[u][d].dedup();
        }
    }
    for co in 0..C {
        for d in 0..2 {
            writers[co][d].sort_unstable();
            writers[co][d].dedup();
        }
    }
}

const fn gcd_i32(mut a: i32, mut b: i32) -> i32 {
    a = a.abs();
    b = b.abs();
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a
}

fn reachability<const S: usize>(adj: &Adj<S>) -> [[bool; S]; S] {
    let mut reach = [[false; S]; S];

    for start in 0..S {
        let mut q = VecDeque::new();
        reach[start][start] = true;
        q.push_back(start);

        while let Some(u) = q.pop_front() {
            for &v in &adj[u] {
                if !reach[start][v] {
                    reach[start][v] = true;
                    q.push_back(v);
                }
            }
        }
    }

    reach
}

/// Color-aware one-sided excursion summary with exact parent/back color.
///
/// `ret[back][st][co]` is a bitmask of states that can be entered by a
/// balanced one-sided computation starting in exact `(st, co)` with immediate
/// parent/back color `back` and finishing by moving back across that boundary.
/// With `clean == true`, every matched pop writes 0, so all cells touched on
/// that side are restored to blank recursively.
///
/// A return has a direct recursive form, so we do not need the old all-pairs
/// `same` transitive closure.  If the first move pops, it returns immediately.
/// If the first move pushes, the child must itself return; after that return we
/// are back on the current cell in the returned state scanning exactly the
/// color printed by the push, and continue from there.  Saturating those
/// return-state masks computes the same grammar with far less work.
struct SideExcursions<const S: usize, const C: usize> {
    // Flattened [back][state][color]. Bit `tr` means a balanced return into
    // state `tr` is possible.
    ret: Vec<u64>,
}

impl<const S: usize, const C: usize> SideExcursions<S, C> {
    const fn node(st: usize, co: usize) -> usize {
        st * C + co
    }

    const fn decode(node: usize) -> (usize, usize) {
        (node / C, node % C)
    }

    const fn ret_index(back: usize, st: usize, co: usize) -> usize {
        (back * S + st) * C + co
    }

    fn ret_states(&self, back: usize, st: usize, co: usize) -> u64 {
        self.ret[Self::ret_index(back, st, co)]
    }

    fn ret_states_from_mask(
        &self,
        back: usize,
        st: usize,
        mut colors: u64,
    ) -> u64 {
        let mut out = 0;
        while colors != 0 {
            let co = colors.trailing_zeros() as usize;
            colors &= colors - 1;
            out |= self.ret_states(back, st, co);
        }
        out
    }

    fn ret_from_mask_possible(
        &self,
        back: usize,
        st: usize,
        colors: u64,
        tr: usize,
    ) -> bool {
        (self.ret_states_from_mask(back, st, colors) & (1_u64 << tr))
            != 0
    }
}

/// Exact possible color mask of the child-side neighbor when the source's
/// parent/back neighbor is known.
fn window_child_mask<const S: usize, const C: usize>(
    state: usize,
    scan: usize,
    push: Shift,
    back: usize,
    possible: &WinPossible<S, C>,
) -> u64 {
    if push {
        possible.right[state][scan][back]
    } else {
        possible.left[state][scan][back]
    }
}

fn side_excursions<const S: usize, const C: usize>(
    prog: &Prog<S, C>,
    windows: &WinPossible<S, C>,
    push: Shift,
    clean: bool,
) -> SideExcursions<S, C> {
    struct PushEq {
        source: usize,
        back: usize,
        print: usize,
    }

    fn add_returns(
        ret: &mut [u64],
        q: &mut VecDeque<(usize, u64)>,
        node: usize,
        bits: u64,
    ) {
        let added = bits & !ret[node];
        if added != 0 {
            ret[node] |= added;
            q.push_back((node, added));
        }
    }

    let pop = !push;
    let ret_len = C * S * C;
    let mut ret = vec![0_u64; ret_len];
    let mut trans = [[None; C]; S];

    for ((st, co), &(print, shift, tr)) in prog.iter() {
        trans[st as usize][co as usize] =
            Some((print as usize, shift, tr as usize));
    }

    // For fixed `push`, both window tables have exactly the indexing we need:
    // [state][scan][known back color] -> possible child colors.
    let child_masks = if push { &windows.right } else { &windows.left };

    let mut pushes = Vec::new();
    let mut child_users = vec![Vec::<usize>::new(); ret_len];
    let mut seeds = Vec::new();

    for back in 0..C {
        for st in 0..S {
            for co in 0..C {
                let child_colors = child_masks[st][co][back];
                if child_colors == 0 {
                    continue;
                }

                let Some((print, shift, tr)) = trans[st][co] else {
                    continue;
                };

                let source =
                    SideExcursions::<S, C>::ret_index(back, st, co);

                if shift == pop {
                    if !clean || print == 0 {
                        seeds.push((source, 1_u64 << tr));
                    }
                    continue;
                }

                let eq = pushes.len();
                pushes.push(PushEq {
                    source,
                    back,
                    print,
                });

                // This push equation depends on the return relation of every
                // child color allowed by the exact forward window.
                let mut colors = child_colors;
                while colors != 0 {
                    let child_co = colors.trailing_zeros() as usize;
                    colors &= colors - 1;
                    let child = SideExcursions::<S, C>::ret_index(
                        print, tr, child_co,
                    );
                    child_users[child].push(eq);
                }
            }
        }
    }

    // Once child return state `r` becomes possible for a push equation, that
    // equation depends on the continuation `(back, r, print)`.  These reverse
    // dependencies are discovered lazily, so each return bit is propagated
    // only to equations that can actually use it.
    let mut continuation_users = vec![Vec::<usize>::new(); ret_len];
    let mut child_returns = vec![0_u64; pushes.len()];
    let mut q = VecDeque::new();

    for (node, bits) in seeds {
        add_returns(&mut ret, &mut q, node, bits);
    }

    while let Some((node, added_states)) = q.pop_front() {
        // `node` is used as a nested child by these equations.  Newly returned
        // states expose newly relevant same-level continuation nodes.
        for &eq_i in &child_users[node] {
            let fresh = added_states & !child_returns[eq_i];
            if fresh == 0 {
                continue;
            }
            child_returns[eq_i] |= fresh;

            let eq = &pushes[eq_i];
            let mut states = fresh;
            while states != 0 {
                let return_st = states.trailing_zeros() as usize;
                states &= states - 1;

                let continuation = SideExcursions::<S, C>::ret_index(
                    eq.back, return_st, eq.print,
                );
                continuation_users[continuation].push(eq_i);

                // The continuation may already have returns from earlier
                // events; consume its full current value when registering.
                let current = ret[continuation];
                add_returns(&mut ret, &mut q, eq.source, current);
            }
        }

        // `node` is a same-level continuation for these equations.  Every new
        // outer return state immediately becomes a return of their sources.
        for &eq_i in &continuation_users[node] {
            let source = pushes[eq_i].source;
            add_returns(&mut ret, &mut q, source, added_states);
        }
    }

    SideExcursions { ret }
}

/// Sound over-approximation of reachable one-sided-blank configurations.
///
/// `blank_side == false` describes `0+ [color] ?`.
/// `blank_side == true`  describes `? [color] 0+`.
///
/// The worklist keeps the blank side clean at abstract checkpoints, but may
/// cross through dirty intermediate configurations via `clean` excursions.
/// The unconstrained side may use arbitrary balanced excursions.
fn halfblank_slots<const S: usize, const C: usize>(
    prog: &Prog<S, C>,
    windows: &WinPossible<S, C>,
    blank_side: Shift,
    clean: &SideExcursions<S, C>,
    away: &SideExcursions<S, C>,
) -> [[bool; C]; S] {
    let mut possible = [[false; C]; S];
    let mut trans = [[None; C]; S];

    for ((st, co), &(print, shift, tr)) in prog.iter() {
        trans[st as usize][co as usize] =
            Some((print as usize, shift, tr as usize));
    }

    let mut q = VecDeque::new();

    #[expect(clippy::shadow_unrelated)]
    let push = |st: usize,
                co: usize,
                possible: &mut [[bool; C]; S],
                q: &mut VecDeque<usize>| {
        let away_side = !blank_side;
        if window_child_mask(st, co, away_side, 0, windows) != 0
            && !possible[st][co]
        {
            possible[st][co] = true;
            q.push_back(SideExcursions::<S, C>::node(st, co));
        }
    };

    push(0, 0, &mut possible, &mut q);

    while let Some(node) = q.pop_front() {
        let (st, co) = SideExcursions::<S, C>::decode(node);

        let Some((print, shift, tr)) = trans[st][co] else {
            continue;
        };

        let away_side = !blank_side;

        if shift == away_side {
            // A complete excursion into the unconstrained side returns to this
            // same boundary without requiring the printed parent cell to be
            // blank.  The source's blank-side/back neighbor is exactly 0; the
            // child's back color is exactly `print`.
            let child_colors =
                window_child_mask(st, co, shift, 0, windows);
            let mut return_states =
                away.ret_states_from_mask(print, tr, child_colors);
            while return_states != 0 {
                let return_st = return_states.trailing_zeros() as usize;
                return_states &= return_states - 1;
                push(return_st, print, &mut possible, &mut q);
            }
        }

        if shift == blank_side {
            // The source's opposite neighbor is on the unconstrained side, so
            // it is not fixed.  Union over that back color only for deciding
            // whether the exact blank child color 0 can occur.
            let child_colors =
                window_neighbor_mask(st, co, shift, windows);
            // The blank-side neighbor is exactly zero.  If zero is not even a
            // possible neighbor in the forward window abstraction, this
            // halfblank checkpoint cannot take this step.
            if child_colors & 1 == 0 {
                continue;
            }

            // Move into the blank side.  The newly scanned cell is exactly 0;
            // the old head joins the unconstrained side, so its print is free.
            push(tr, 0, &mut possible, &mut q);

            // Or make a complete clean excursion into that side and return to
            // the same boundary.  The child starts in exact color 0.
            // Once we enter the child, its parent/back cell contains the
            // exact color printed by the departure transition.
            let mut return_states = clean.ret_states(print, tr, 0);
            while return_states != 0 {
                let return_st = return_states.trailing_zeros() as usize;
                return_states &= return_states - 1;
                push(return_st, print, &mut possible, &mut q);
            }
        } else if print == 0 {
            // Move away from the blank side.  The old head cell joins that
            // side and must be left as 0.  Since the blank-side neighbor of
            // the source is exactly 0, condition the forward window on that
            // exact parent/back color instead of unioning over all neighbors.
            let mut colors =
                window_child_mask(st, co, shift, 0, windows);
            while colors != 0 {
                let out_co = colors.trailing_zeros() as usize;
                colors &= colors - 1;
                push(tr, out_co, &mut possible, &mut q);
            }
        }
    }

    possible
}

fn scc_from_reach<const S: usize>(
    reach: &[[bool; S]; S],
) -> ([usize; S], [u16; S], usize) {
    let mut comp = [usize::MAX; S];
    let mut masks = [0; S];
    let mut k = 0;

    for i in 0..S {
        if comp[i] != usize::MAX {
            continue;
        }
        let cid = k;
        k += 1;

        let mut mask: u16 = 0;
        for j in 0..S {
            if reach[i][j] && reach[j][i] {
                comp[j] = cid;
                mask |= 1 << j;
            }
        }
        masks[cid] = mask;
    }

    (comp, masks, k)
}

fn add_gen<const S: usize>(arr: &mut [i32; S], len: &mut u8, val: i32) {
    debug_assert!(val > 0);
    let n = *len as usize;
    for i in 0..n {
        if arr[i] == val {
            return;
        }
    }
    if n < S {
        arr[n] = val;
        *len += 1;
    }
}

/// DC meta + generators.
/// Returns:
/// - reach
/// - comp[state]
/// - masks[cid]
/// - k
/// - g_scc[cid]
/// - res[state]
/// - pos_gens[cid], pos_len[cid] : positive cycle displacements found
/// - neg_gens[cid], neg_len[cid] : absolute value of negative cycle displacements found
#[expect(clippy::excessive_nesting)]
fn dc_meta_with_gens<const S: usize>(
    adj: &Adj<S>,
    next: &NextDir<S>,
) -> (
    [[bool; S]; S],
    [usize; S],
    [u16; S],
    usize,
    [i32; S],
    [i32; S],
    [[i32; S]; S],
    [u8; S],
    [[i32; S]; S],
    [u8; S],
) {
    let reach = reachability::<S>(adj);
    let (comp, masks, k) = scc_from_reach::<S>(&reach);

    let mut g_scc = [0; S];
    let mut res = [0; S];

    let mut pos_gens = [[0; S]; S];
    let mut pos_len = [0; S];
    let mut neg_gens = [[0; S]; S];
    let mut neg_len = [0; S];

    for cid in 0..k {
        let mask = masks[cid];
        if mask == 0 {
            continue;
        }

        let Some(root) = (0..S).find(|&v| (mask >> v) & 1 != 0) else {
            continue;
        };

        let in_comp: [bool; S] = from_fn(|v| ((mask >> v) & 1) == 1);

        let mut dist: [Option<i32>; S] = [None; S];
        dist[root] = Some(0);

        let mut q = VecDeque::new();
        q.push_back(root);

        let mut g = 0;

        while let Some(u) = q.pop_front() {
            let du = dist[u].unwrap();

            for dir in 0..2 {
                let w = if dir == 1 { 1 } else { -1 }; // R:+1, L:-1

                for &v in &next[u][dir] {
                    if !in_comp[v] {
                        continue;
                    }

                    let dv_new = du + w;

                    match dist[v] {
                        None => {
                            dist[v] = Some(dv_new);
                            q.push_back(v);
                        },
                        Some(dv) => {
                            // discrepancy = closed-walk displacement
                            let diff = dv_new - dv;
                            if diff != 0 {
                                g = if g == 0 {
                                    diff.abs()
                                } else {
                                    gcd_i32(g, diff)
                                };

                                if diff > 0 {
                                    add_gen::<S>(
                                        &mut pos_gens[cid],
                                        &mut pos_len[cid],
                                        diff,
                                    );
                                } else {
                                    add_gen::<S>(
                                        &mut neg_gens[cid],
                                        &mut neg_len[cid],
                                        -diff,
                                    );
                                }
                            }
                        },
                    }
                }
            }
        }

        g_scc[cid] = g;

        // fill residues
        for v in 0..S {
            if !in_comp[v] {
                continue;
            }
            let dv = dist[v].unwrap_or(0);
            res[v] = if g == 0 {
                dv
            } else {
                let mut r = dv % g;
                if r < 0 {
                    r += g;
                }
                r
            };
        }
    }

    (
        reach, comp, masks, k, g_scc, res, pos_gens, pos_len, neg_gens,
        neg_len,
    )
}

/// Bellman-Ford negative-cycle detection inside SCC.
/// If `negate` is true, weights are negated => detects positive
/// cycles of original graph.
fn has_neg_cycle_in_scc<const S: usize>(
    mask: u16,
    next: &NextDir<S>,
    negate: bool,
) -> bool {
    let mut nodes = [0; S];
    let mut n = 0;
    for v in 0..S {
        if ((mask >> v) & 1) == 1 {
            nodes[n] = v;
            n += 1;
        }
    }
    if n == 0 {
        return false;
    }

    let mut dist = [0; S];

    for iter in 0..n {
        let mut changed = false;

        for i in 0..n {
            let u = nodes[i];
            let du = dist[u];

            for dir in 0..2 {
                let mut w = if dir == 1 { 1 } else { -1 };
                if negate {
                    w = -w;
                }

                for &v in &next[u][dir] {
                    if ((mask >> v) & 1) == 0 {
                        continue;
                    }
                    let nv = du + w;
                    if nv < dist[v] {
                        dist[v] = nv;
                        changed = true;
                    }
                }
            }
        }

        if !changed {
            return false;
        }
        if iter == n - 1 && changed {
            return true;
        }
    }

    false
}

type ColorMask = u64;

fn printed_mask<const S: usize, const C: usize>(
    prog: &Prog<S, C>,
) -> ColorMask {
    let mut m = 0;
    for ((_, _read), &(pr, _, _)) in prog.iter() {
        let pr = pr as usize;
        if pr < C {
            m |= 1 << pr;
        }
    }
    m
}

fn color_closure<const S: usize, const C: usize>(
    prog: &Prog<S, C>,
) -> [ColorMask; C] {
    debug_assert!(C <= 64);

    let mut clo = [0; C];

    // direct edges: read -> print
    for ((_, read), &(pr, _, _)) in prog.iter() {
        let a = read as usize;
        let b = pr as usize;
        if a < C && b < C {
            clo[a] |= 1 << b;
        }
    }

    // include self
    for a in 0..C {
        clo[a] |= 1 << a;
    }

    // transitive closure (bitset Floyd)
    for k in 0..C {
        let kset = clo[k];
        for a in 0..C {
            if ((clo[a] >> k) & 1) != 0 {
                clo[a] |= kset;
            }
        }
    }

    clo
}

fn unerasable_mask<const C: usize>(clo: &[ColorMask; C]) -> ColorMask {
    // bit i set => color i>0 cannot reach 0
    let mut m = 0;
    for a in 1..C {
        let can0 = (clo[a] & 1) != 0; // bit0 is color 0
        if !can0 {
            m |= 1 << a;
        }
    }
    m
}

impl<const S: usize, const C: usize> Prog<S, C> {
    fn entrypoints_and_indices(&self) -> (Entrypoints, Indices<S, C>) {
        let mut entrypoints = Entrypoints::new();
        let mut idx = indices_new::<S, C>();

        for (slot @ (st, _), &(pr, sh, tr)) in self.iter() {
            let (same, diff) = entrypoints.entry(tr).or_default();

            (if st == tr { same } else { diff }).push((slot, (pr, sh)));

            indices_add::<S, C>(&mut idx, st, tr, sh, pr);
        }

        indices_finalize::<S, C>(&mut idx);

        (entrypoints, idx)
    }

    /// Static halt-slot filter:
    /// - reachability + SCC residue gate (DC)
    /// - if SCC has both drift signs: keep conservative
    /// - if SCC is one-sided: do an *exact* “can we hit net displacement 0?” check
    ///   via a small bounded product-graph BFS (state × displacement window).
    #[expect(clippy::excessive_nesting)]
    pub fn halt_slots_disp_side(
        &self,
        idx: &Indices<S, C>,
    ) -> Set<Slot> {
        let (adj, preds, writers, next) = idx;

        let (
            reach,
            comp,
            masks,
            k,
            _g_scc,
            res,
            _pos_gens,
            _pos_len,
            _neg_gens,
            _neg_len,
        ) = dc_meta_with_gens::<S>(adj, next);

        // SCC drift classification (same as you already do)
        let mut has_neg = [false; S];
        let mut has_pos = [false; S];
        for cid in 0..k {
            let mask = masks[cid];
            has_neg[cid] = has_neg_cycle_in_scc::<S>(mask, next, false);
            has_pos[cid] = has_neg_cycle_in_scc::<S>(mask, next, true);
        }

        // NEW: exact 0-displacement reachability cache for one-sided SCCs.
        // zero_done[cid][src] indicates whether we computed zero_reach[cid][src].
        // zero_reach[cid][src] is bitmask of nodes reachable from src with net disp 0
        // (under an orientation where SCC has no negative cycles).
        let mut zero_done = [[false; S]; S];
        let mut zero_reach = [[0; S]; S];

        let (max_st, max_co) = self.max_reached();

        (0..=max_st)
            .flat_map(|st| (0..=max_co).map(move |co| (st, co)))
            .filter(|slot @ &(st, co)| {
                // only consider missing slots as "candidate halting slots"
                self.get(slot).is_none()
                    && (co == 0 || {
                        let h = st as usize;
                        let co = co as usize;
                        if h >= S || co >= C {
                            return false;
                        }

                        for w in 0..2 {
                            let need = w ^ 1;

                            for &p in &preds[h][need] {
                                for &s0 in &writers[co][w] {
                                    if !reach[s0][p] {
                                        continue;
                                    }

                                    // across SCCs: conservative keep
                                    if comp[s0] != comp[p] {
                                        return true;
                                    }

                                    // same SCC
                                    let cid = comp[p];
                                    if cid >= k {
                                        return true; // conservative
                                    }

                                    // residue gate (necessary; conservative if weak)
                                    if res[s0] != res[p] {
                                        continue;
                                    }

                                    // If SCC has both signs, congruence is about all we can use cheaply;
                                    // keep witness.
                                    if has_pos[cid] && has_neg[cid] {
                                        return true;
                                    }

                                    // SCC is one-sided (or bounded). Do exact disp==0 reachability.
                                    // Choose an orientation with NO negative cycles:
                                    // - if SCC has no neg cycles, use normal weights (R=+1,L=-1)
                                    // - if SCC has neg cycles but no pos cycles, negate weights
                                    let negate = has_neg[cid] && !has_pos[cid];

                                    if !zero_done[cid][s0] {
                                        zero_reach[cid][s0] =
                                            zero_disp_reach_mask_one_sided_scc::<S>(
                                                masks[cid],
                                                next,
                                                s0,
                                                negate,
                                            );
                                        zero_done[cid][s0] = true;
                                    }

                                    // p reachable from s0 with net displacement 0?
                                    if ((zero_reach[cid][s0] >> p) & 1) == 0 {
                                        // No exact 0-displacement witness in this SCC => prune this witness
                                        continue;
                                    }

                                    // Exact witness exists => keep candidate halt slot
                                    return true;
                                }
                            }
                        }

                        false
                    })
            })
            .collect()
    }

    /// Strengthen candidate halt slots with the color-aware one-sided
    /// excursion relation.
    ///
    /// For a nonblank scanned color, take the last departure from the eventual
    /// halting cell.  That transition must write the halting color and move
    /// into one side; until the final return, the head stays strictly on that
    /// side, so an ordinary balanced excursion from the entered child state
    /// must be able to return into the halting state.
    ///
    /// A halt scanning 0 has one additional possibility: the cell may be a
    /// first visit to a fresh blank frontier.  Such a configuration necessarily
    /// has one whole side blank, which is exactly what the halfblank abstraction
    /// records.  Previously visited zero cells are covered by the same
    /// last-departure rule, with a transition that writes 0.
    fn halt_slots_side_excursion(&self, slots: Set<Slot>) -> Set<Slot> {
        if slots.is_empty() {
            return slots;
        }

        let (forbid_left, forbid_right) = self.shift_side_forbidden();
        let windows =
            self.win_possible_from_blank(&forbid_left, &forbid_right);
        let left_any = side_excursions(self, &windows, false, false);
        let right_any = side_excursions(self, &windows, true, false);

        let halfblank =
            slots.iter().any(|&(_, color)| color == 0).then(|| {
                let left_clean =
                    side_excursions(self, &windows, false, true);
                let right_clean =
                    side_excursions(self, &windows, true, true);
                let left_half = halfblank_slots(
                    self,
                    &windows,
                    false,
                    &left_clean,
                    &right_any,
                );
                let right_half = halfblank_slots(
                    self,
                    &windows,
                    true,
                    &right_clean,
                    &left_any,
                );
                (left_half, right_half)
            });

        slots
            .into_iter()
            .filter(|&(state, color)| {
                let h = state as usize;
                let co = color as usize;

                if !windows.any[h][co] {
                    return false;
                }

                if color == 0
                    && let Some((left_half, right_half)) = &halfblank
                    && (left_half[h][0] || right_half[h][0])
                {
                    return true;
                }

                self.iter().any(
                    |((st, read), &(print, shift, child_st))| {
                        if print != color {
                            return false;
                        }

                        let st = st as usize;
                        let read = read as usize;
                        let child_st = child_st as usize;
                        let child_colors = window_neighbor_mask(
                            st, read, shift, &windows,
                        );
                        let excursions =
                            if shift { &right_any } else { &left_any };

                        excursions.ret_from_mask_possible(
                            co,
                            child_st,
                            child_colors,
                            h,
                        )
                    },
                )
            })
            .collect()
    }

    /// Static target-shape filter for `0+ [color] 0+` blank predecessors.
    ///
    /// `halfblank_slots` tracks the stronger necessary condition that one
    /// whole side is blank in a reachable `(state, scanned color)` checkpoint.
    /// Every exact blank target must occur in both the left-blank and
    /// right-blank abstractions.
    ///
    /// For a nonblank scanned color, also take the last departure from that
    /// target cell.  The untouched opposite side must already be blank at the
    /// departure, replacing the old weak `control state is reachable` gate.
    /// The departed side must then admit a clean return to the target state.
    fn blank_slots_side_clean(&self) -> Set<Slot> {
        let (forbid_left, forbid_right) = self.shift_side_forbidden();
        let windows =
            self.win_possible_from_blank(&forbid_left, &forbid_right);
        let left_clean = side_excursions(self, &windows, false, true);
        let right_clean = side_excursions(self, &windows, true, true);
        let left_any = side_excursions(self, &windows, false, false);
        let right_any = side_excursions(self, &windows, true, false);

        // false = left side blank:  0+ [color] ?
        // true  = right side blank: ? [color] 0+
        let left_half = halfblank_slots(
            self,
            &windows,
            false,
            &left_clean,
            &right_any,
        );
        let right_half = halfblank_slots(
            self,
            &windows,
            true,
            &right_clean,
            &left_any,
        );

        self.erase_slots()
            .into_iter()
            .filter(|&(state, color)| {
                let h = state as usize;
                let co = color as usize;

                // An exact `0+ [color] 0+` occurrence witnesses both
                // one-sided abstractions on the same concrete run.
                if !left_half[h][co] || !right_half[h][co] {
                    return false;
                }

                // A scanned 0 may be a first visit to a fresh blank cell, so
                // there need not be an earlier departure from this cell.
                if color == 0 {
                    return true;
                }

                // For a nonzero scanned color, the cell was written earlier.
                // At its last departure before the target, the opposite side
                // is never touched again and therefore must already be blank.
                for ((st, read), &(print, shift, child_st)) in
                    self.iter()
                {
                    let st = st as usize;
                    let read = read as usize;
                    let child_st = child_st as usize;
                    if print != color {
                        continue;
                    }

                    let (opposite_half, clean) = if shift {
                        // Depart right: left side remains untouched.
                        (&left_half, &right_clean)
                    } else {
                        // Depart left: right side remains untouched.
                        (&right_half, &left_clean)
                    };

                    if !opposite_half[st][read] {
                        continue;
                    }

                    let child_colors =
                        window_child_mask(st, read, shift, 0, &windows);
                    if clean.ret_from_mask_possible(
                        co,
                        child_st,
                        child_colors,
                        h,
                    ) {
                        return true;
                    }
                }

                false
            })
            .collect()
    }

    /// Filter one-sided zero targets by reachable halfblank shape:
    ///
    ///  ? [0] 0+  (side = R)
    ///  0+ [0] ?  (side = L)
    fn shifts_side_clean(
        &self,
        shifts: Set<(State, Shift)>,
    ) -> Set<(State, Shift)> {
        let (forbid_left, forbid_right) = self.shift_side_forbidden();
        let windows =
            self.win_possible_from_blank(&forbid_left, &forbid_right);
        let left_clean = side_excursions(self, &windows, false, true);
        let right_clean = side_excursions(self, &windows, true, true);
        let left_any = side_excursions(self, &windows, false, false);
        let right_any = side_excursions(self, &windows, true, false);
        let left_half = halfblank_slots(
            self,
            &windows,
            false,
            &left_clean,
            &right_any,
        );
        let right_half = halfblank_slots(
            self,
            &windows,
            true,
            &right_clean,
            &left_any,
        );

        shifts
            .into_iter()
            .filter(|&(state, side)| {
                let h = state as usize;
                if side {
                    right_half[h][0]
                } else {
                    left_half[h][0]
                }
            })
            .collect()
    }

    fn spinout_shifts_side_clean(&self) -> Set<(State, Shift)> {
        self.shifts_side_clean(self.zr_shifts())
    }

    fn zloop_shifts_side_clean(&self) -> Set<(State, Shift)> {
        self.shifts_side_clean(self.blank_loops())
    }

    fn cant_blank_by_color_graph(&self) -> bool {
        let clo = color_closure::<S, C>(self);
        let bad = unerasable_mask::<C>(&clo);
        if bad == 0 {
            return false;
        }

        let pr = printed_mask::<S, C>(self);

        (pr & bad) != 0
    }
}

/// BF min distances inside SCC with optional weight negation.
/// If `negate=true`, weights are negated (R=-1, L=+1).
fn bf_min_row_in_scc_weight<const S: usize>(
    mask: u16,
    next: &NextDir<S>,
    src: usize,
    negate: bool,
    out: &mut [i32; S],
) {
    const INF: i32 = 1_000_000;

    *out = [INF; S];
    out[src] = 0;

    let mut nodes = [0; S];
    let mut n = 0;
    for v in 0..S {
        if ((mask >> v) & 1) == 1 {
            nodes[n] = v;
            n += 1;
        }
    }
    if n == 0 {
        return;
    }

    for _ in 0..(n.saturating_sub(1)) {
        let mut changed = false;

        for i in 0..n {
            let u = nodes[i];
            let du = out[u];
            if du == INF {
                continue;
            }

            for dir in 0..2 {
                let mut w = if dir == 1 { 1 } else { -1 };
                if negate {
                    w = -w;
                }

                for &v in &next[u][dir] {
                    if ((mask >> v) & 1) == 0 {
                        continue;
                    }
                    let nv = du + w;
                    if nv < out[v] {
                        out[v] = nv;
                        changed = true;
                    }
                }
            }
        }

        if !changed {
            break;
        }
    }
}

/// Exact check inside a one-sided SCC:
/// Return bitmask of states v in SCC such that there exists a path src -> v
/// with net displacement exactly 0, under weights:
/// - normal: R=+1, L=-1 if negate=false
/// - negated: R=-1, L=+1 if negate=true
///
/// Assumes: under the chosen weight system, SCC has no negative cycles
/// (so min distance is bounded and the explored displacement window is small).
fn zero_disp_reach_mask_one_sided_scc<const S: usize>(
    mask: u16,
    next: &NextDir<S>,
    src: usize,
    negate: bool,
) -> u16 {
    // Compute global lower bound on displacement reachable from src in SCC:
    // min over nodes of shortest path distance (no negative cycles => finite).
    let mut d = [0; S];
    bf_min_row_in_scc_weight::<S>(mask, next, src, negate, &mut d);

    let lo_opt = (0..S)
        .filter(|&v| (mask >> v) & 1 != 0)
        .map(|v| d[v])
        .filter(|&dv| dv < 900_000)
        .min();

    let Some(lo) = lo_opt else { return 0 };

    // We need to search displacements in [lo .. 0].
    // For S<=16 and no negative cycles, lo is typically >= -(S-1) (<= -15).
    // Keep a safe cap; if it somehow exceeds the cap, return
    // conservative "all nodes".
    const CAP: usize = 33; // supports lo down to -32
    let offset = -lo;
    #[expect(clippy::cast_sign_loss)]
    if offset < 0 || (offset as usize) >= CAP {
        // Too wide; don't prune.
        return mask;
    }
    #[expect(clippy::cast_sign_loss)]
    let zero_idx = offset as usize; // index representing displacement 0

    // visited[state][idx] where idx corresponds to disp = lo + idx
    let mut visited = [[false; CAP]; S];

    let mut q = VecDeque::new();
    visited[src][zero_idx] = true;
    q.push_back((src, 0)); // store actual displacement

    while let Some((u, disp)) = q.pop_front() {
        for dir in 0..2 {
            let mut w = if dir == 1 { 1 } else { -1 };
            if negate {
                w = -w;
            }

            for &v in &next[u][dir] {
                if ((mask >> v) & 1) == 0 {
                    continue;
                }
                let nd = disp + w;
                if nd < lo || nd > 0 {
                    continue;
                }
                #[expect(clippy::cast_sign_loss)]
                let idx = (nd - lo) as usize;
                if idx >= CAP || visited[v][idx] {
                    continue;
                }
                visited[v][idx] = true;
                q.push_back((v, nd));
            }
        }
    }

    // Collect targets reachable with displacement exactly 0
    let mut out = 0;
    for v in 0..S {
        if ((mask >> v) & 1) == 0 {
            continue;
        }
        if visited[v][zero_idx] {
            out |= 1 << v;
        }
    }
    out
}

#[expect(clippy::multiple_inherent_impl)]
impl Span {
    /// Compare two block-spans from the head outward.
    ///
    /// Exact counts match exactly. `AtLeast(a)` subsumes `Exact(b)` and
    /// `AtLeast(b)` precisely when `a <= b`. If self runs out of blocks, it
    /// only subsumes a longer span when its end is Unknown.
    fn subsumes(&self, other: &Self) -> bool {
        // These cheap length/end checks previously lived in
        // `maybe_subsumes`, causing the leading runs to be examined twice.
        if self.end == TapeEnd::Blanks && other.end == TapeEnd::Unknown
        {
            return false;
        }

        let self_len = self.span.len();
        let other_len = other.span.len();

        if self_len > other_len {
            return false;
        }
        if self_len < other_len && self.end == TapeEnd::Blanks {
            return false;
        }

        for (a, b) in self.span.iter().zip(other.span.iter()) {
            if a.color != b.color {
                return false;
            }
            if !a.count.subsumes(b.count) {
                return false;
            }
        }

        true
    }
}

/**************************************/

const PERIODIC_MIN_PERIOD: usize = 2;
const PERIODIC_MAX_PERIOD: usize = 7;
const PERIODIC_MIN_PAIRS_PER_PHASE: usize = 2;
const PERIODIC_MAX_NEED: usize =
    PERIODIC_MAX_PERIOD * (PERIODIC_MIN_PAIRS_PER_PHASE + 1);

const fn periodic_periods() -> core::ops::RangeInclusive<usize> {
    PERIODIC_MIN_PERIOD..=PERIODIC_MAX_PERIOD
}

const fn periodic_need(period: usize) -> usize {
    period * (PERIODIC_MIN_PAIRS_PER_PHASE + 1)
}

/**************************************/

/// A periodic-growth closer used for both:
///
/// - linear branches, represented as a one-config frontier; and
/// - full frontier snapshots, represented as a sorted list of configs.
///
/// The two callers still keep separate histories because they observe different
/// streams, but the certificate is identical: for each phase of a candidate
/// period, every sampled `snapshot[n] -> snapshot[n + period]` pair must have
/// the same growth signature.
#[derive(Default)]
struct PeriodicHistory {
    snaps: Vec<PeriodicSnap>,
}

type FastFrontier = Arc<[FastCfg]>;

#[derive(Clone)]
struct PeriodicSnap {
    step: Steps,
    front: FastFrontier,
}

impl PeriodicHistory {
    // Keep more than the detection window so that, after a successful frontier
    // detection, we can extend the detected growth pattern to the left and
    // recover the earliest stable frontier in recent history.
    const KEEP: usize = PERIODIC_MAX_NEED * 2 + 6;

    // Keep this conservative.  If a frontier is huge, the closer should not
    // become the bottleneck; ordinary antichain/search limits can handle it.
    // Raising this is safe but may cost time on very wide halt cones.
    const MAX_FRONTIER_FOR_CLOSER: usize = 20_000;

    fn clear(&mut self) {
        self.snaps.clear();
    }

    fn push_and_detect(&mut self, cfg: &Config) -> bool {
        self.push_snap(
            0,
            Arc::<[FastCfg]>::from(vec![FastCfg::from_config(cfg)]),
        );
        self.detect_any_phase_growth().is_some()
    }

    fn observe_frontier(
        &mut self,
        step: Steps,
        front: FastFrontier,
    ) -> Option<Steps> {
        if front.is_empty() {
            return None;
        }

        if front.len() > Self::MAX_FRONTIER_FOR_CLOSER {
            self.clear();
            return None;
        }

        self.push_snap(step, front);

        let cycle_start_idx = self.detect_any_phase_growth()?;
        Some(self.snaps[cycle_start_idx].step)
    }

    fn push_snap(&mut self, step: Steps, front: FastFrontier) {
        self.snaps.push(PeriodicSnap { step, front });
        if self.snaps.len() > Self::KEEP {
            self.snaps.remove(0);
        }
    }

    fn detect_any_phase_growth(&self) -> Option<usize> {
        for period in periodic_periods() {
            let need = periodic_need(period);
            if self.snaps.len() < need {
                continue;
            }

            let start = self.snaps.len() - need;
            if let Some(cycle_start) =
                self.detect_period_growth_from(start, period)
            {
                return Some(cycle_start);
            }
        }
        None
    }

    fn detect_period_growth_from(
        &self,
        start: usize,
        period: usize,
    ) -> Option<usize> {
        let need = periodic_need(period);
        if start + need > self.snaps.len() {
            return None;
        }

        // Periods greater than two are useful for clean phase ladders such as
        //     A_n -> B_n -> C_{n+1} -> A_{n+1}
        // but a repeating split/merge frontier, e.g. widths 1,2,1, can be the
        // reverse image of a finite forward countdown rather than a genuine
        // closed periodic cone.  Keep multi-phase frontier certificates only
        // when every sampled phase has the same frontier width.
        if period > 2 {
            let width = self.snaps[start].front.len();
            if self.snaps[start..start + need]
                .iter()
                .any(|snap| snap.front.len() != width)
            {
                return None;
            }
        }

        let expected =
            self.expected_period_signatures(start, period)?;

        // Verify the detection window.
        for j in start..start + need - period {
            let sig = Self::frontier_growth_signature(
                &self.snaps[j].front,
                &self.snaps[j + period].front,
            )?;
            if sig != expected[(j - start) % period] {
                return None;
            }
        }

        // Walk left as far as the same absolute phase signatures continue to
        // hold.  This recovers the first retained stable frontier, not merely
        // the first frontier in the detection window.
        let mut cycle_start = start;
        while cycle_start > 0 {
            let candidate = cycle_start - 1;
            if candidate + period >= self.snaps.len() {
                break;
            }

            let Some(sig) = Self::frontier_growth_signature(
                &self.snaps[candidate].front,
                &self.snaps[candidate + period].front,
            ) else {
                break;
            };

            if sig != expected[(candidate + period - start) % period] {
                break;
            }

            cycle_start = candidate;
        }

        Some(cycle_start)
    }

    fn expected_period_signatures(
        &self,
        start: usize,
        period: usize,
    ) -> Option<Vec<Vec<BranchSig>>> {
        let need = periodic_need(period);
        let end = start + need;
        if end > self.snaps.len() {
            return None;
        }

        let mut expected: Vec<Option<Vec<BranchSig>>> =
            vec![None; period];
        let mut counts = vec![0; period];

        for j in start..end.saturating_sub(period) {
            let sig = Self::frontier_growth_signature(
                &self.snaps[j].front,
                &self.snaps[j + period].front,
            )?;
            if sig.is_empty() {
                return None;
            }

            let phase = (j - start) % period;
            match &expected[phase] {
                None => expected[phase] = Some(sig),
                Some(prev) if *prev == sig => {},
                Some(_) => return None,
            }
            counts[phase] += 1;
        }

        if counts
            .iter()
            .any(|&count| count < PERIODIC_MIN_PAIRS_PER_PHASE)
        {
            return None;
        }

        expected.into_iter().collect()
    }

    fn frontier_growth_signature(
        a: &[FastCfg],
        b: &[FastCfg],
    ) -> Option<Vec<BranchSig>> {
        if a.is_empty() || a.len() != b.len() {
            return None;
        }

        // Stored snapshots are canonicalized once before insertion.  Do not
        // clone and re-sort them for every candidate period/pair.

        let mut index: Dict<BucketKey, Vec<usize>> = Dict::new();
        for (j, cb) in b.iter().enumerate() {
            // Candidate where the left side grows and the right side is fixed.
            index
                .entry(BucketKey {
                    state: cb.state,
                    scan: cb.scan,
                    grow_side: Side::Left,
                    grow_end: cb.l_end,
                    same_end: cb.r_end,
                    same: cb.right.clone(),
                })
                .or_default()
                .push(j);

            // Candidate where the right side grows and the left side is fixed.
            index
                .entry(BucketKey {
                    state: cb.state,
                    scan: cb.scan,
                    grow_side: Side::Right,
                    grow_end: cb.r_end,
                    same_end: cb.l_end,
                    same: cb.left.clone(),
                })
                .or_default()
                .push(j);
        }

        let mut used = vec![false; b.len()];
        let mut out = Vec::with_capacity(a.len());

        for ca in a {
            let mut found: Option<(usize, BranchSig)> = None;

            // Try left-growth candidates.
            let lkey = BucketKey {
                state: ca.state,
                scan: ca.scan,
                grow_side: Side::Left,
                grow_end: ca.l_end,
                same_end: ca.r_end,
                same: ca.right.clone(),
            };
            if let Some(cands) = index.get(&lkey) {
                for &j in cands {
                    if used[j] {
                        continue;
                    }
                    let cb = &b[j];
                    // `lspan` is stored nearest-head first, while display
                    // prints it in reverse.  The common blank-growth ladder
                    // grows at the far/outer end of this stored vector, e.g.
                    //
                    //     [0, 1^n] -> [0, 1^(n+1)]
                    //
                    // but some backward chains grow at the near/head end.
                    // Try far growth first, then near growth as an additive
                    // fallback so old detections are preserved.
                    if let Some((grow_pos, anchor, delta)) =
                        left_growth_delta(&ca.left, &cb.left)
                    {
                        let sig = BranchSig {
                            state: ca.state,
                            scan: ca.scan,
                            dh: cb.head - ca.head,
                            grow_side: Side::Left,
                            grow_pos,
                            grow_end: ca.l_end,
                            grow_anchor: anchor,
                            grow_delta: delta,
                            same_end: ca.r_end,
                            same: ca.right.clone(),
                        };
                        match &found {
                            None => found = Some((j, sig)),
                            Some((_, prev)) if *prev == sig => {},
                            Some(_) => return None,
                        }
                    }
                }
            }

            // Try right-growth candidates.
            let rkey = BucketKey {
                state: ca.state,
                scan: ca.scan,
                grow_side: Side::Right,
                grow_end: ca.r_end,
                same_end: ca.l_end,
                same: ca.left.clone(),
            };
            if let Some(cands) = index.get(&rkey) {
                for &j in cands {
                    if used[j] {
                        continue;
                    }
                    let cb = &b[j];
                    if let Some((grow_pos, anchor, delta)) =
                        right_growth_delta(&ca.right, &cb.right)
                    {
                        let sig = BranchSig {
                            state: ca.state,
                            scan: ca.scan,
                            dh: cb.head - ca.head,
                            grow_side: Side::Right,
                            grow_pos,
                            grow_end: ca.r_end,
                            grow_anchor: anchor,
                            grow_delta: delta,
                            same_end: ca.l_end,
                            same: ca.left.clone(),
                        };
                        match &found {
                            None => found = Some((j, sig)),
                            Some((_, prev)) if *prev == sig => {},
                            Some(_) => return None,
                        }
                    }
                }
            }

            let (j, sig) = found?;
            used[j] = true;
            out.push(sig);
        }

        if used.iter().any(|&x| !x) {
            return None;
        }

        out.sort_unstable();
        Some(out)
    }
}

fn sorted_fast_frontier(frontier: &[Config]) -> FastFrontier {
    let mut front: Vec<FastCfg> =
        frontier.iter().map(FastCfg::from_config).collect();
    front.sort_unstable();
    Arc::from(front)
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct FastCfg {
    state: State,
    scan: Color,
    head: Pos,
    l_end: EndSig,
    r_end: EndSig,
    left: Vec<RunSig>,
    right: Vec<RunSig>,
}

impl FastCfg {
    fn from_config(cfg: &Config) -> Self {
        Self {
            state: cfg.state,
            scan: cfg.tape.scan,
            head: cfg.tape.head,
            l_end: EndSig::from_end(&cfg.tape.lspan.end),
            r_end: EndSig::from_end(&cfg.tape.rspan.end),
            left: span_runs(&cfg.tape.lspan),
            right: span_runs(&cfg.tape.rspan),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct RunSig {
    color: Color,
    count: BlockCount,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum EndSig {
    Blanks,
    Unknown,
}

impl EndSig {
    const fn from_end(end: &TapeEnd) -> Self {
        match end {
            TapeEnd::Blanks => Self::Blanks,
            TapeEnd::Unknown => Self::Unknown,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Side {
    Left,
    Right,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum GrowthPos {
    // Growth is adjacent to the finite context/head side of the stored span.
    Near,
    // Growth is adjacent to the tape end side of the stored span.
    Far,
    // Growth is inserted immediately before a stable far-end suffix.
    // This catches ladders such as
    //     0 1 0 1 0 1^2 -> 0 1 0 1 0 1 0 1^2
    // where the repeated block grows in front of a terminal marker rather
    // than at the absolute tape end.
    BeforeStableSuffix,
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct BucketKey {
    state: State,
    scan: Color,
    grow_side: Side,
    grow_end: EndSig,
    same_end: EndSig,
    same: Vec<RunSig>,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct BranchSig {
    state: State,
    scan: Color,
    dh: Pos,
    grow_side: Side,
    grow_pos: GrowthPos,
    grow_end: EndSig,
    // Empty for ordinary near/far-end growth.  For BeforeStableSuffix this is
    // the far-end suffix that remains fixed while the middle block grows.
    grow_anchor: Vec<RunSig>,
    grow_delta: Vec<RunSig>,
    same_end: EndSig,
    same: Vec<RunSig>,
}

fn span_runs(span: &Span) -> Vec<RunSig> {
    let mut out = Vec::new();
    for b in span.span.iter() {
        out.push(RunSig {
            color: b.color,
            count: b.count,
        });
    }
    out
}

fn left_growth_delta(
    old: &[RunSig],
    new: &[RunSig],
) -> Option<(GrowthPos, Vec<RunSig>, Vec<RunSig>)> {
    // Spans are stored nearest-head first.  Far growth is the usual outward
    // blank-ladder case: old is a prefix of new, with delta at the tape-end
    // side.  Near growth is retained as a fallback for older detections.
    suffix_after_prefix_runs(old, new)
        .map(|delta| (GrowthPos::Far, Vec::new(), delta))
        .or_else(|| {
            prefix_before_suffix_runs(old, new)
                .map(|delta| (GrowthPos::Near, Vec::new(), delta))
        })
        .or_else(|| middle_growth_before_stable_suffix(old, new))
}

fn right_growth_delta(
    old: &[RunSig],
    new: &[RunSig],
) -> Option<(GrowthPos, Vec<RunSig>, Vec<RunSig>)> {
    // Right spans use the same nearest-head-first storage.  Prefer far/outward
    // growth, then fall back to near/head-side growth.
    suffix_after_prefix_runs(old, new)
        .map(|delta| (GrowthPos::Far, Vec::new(), delta))
        .or_else(|| {
            prefix_before_suffix_runs(old, new)
                .map(|delta| (GrowthPos::Near, Vec::new(), delta))
        })
        .or_else(|| middle_growth_before_stable_suffix(old, new))
}

fn middle_growth_before_stable_suffix(
    old: &[RunSig],
    new: &[RunSig],
) -> Option<(GrowthPos, Vec<RunSig>, Vec<RunSig>)> {
    // Some halt cones grow by inserting a repeated block in front of a stable
    // terminal marker rather than by extending the absolute end of the span:
    //
    //     P S -> P D S -> P D D S -> ...
    //
    // where S is a fixed far-end suffix.  The normal prefix/suffix growth
    // tests reject this because the whole old span is neither a prefix nor a
    // suffix of the new span.  Try every non-empty far suffix S and require
    // the inserted middle delta D to be non-empty.
    if old.len() < 2 {
        return None;
    }

    for split in (1..old.len()).rev() {
        let prefix = &old[..split];
        let suffix = &old[split..];
        let Some(after_prefix) =
            remainder_after_prefix_runs(prefix, new)
        else {
            continue;
        };
        let Some(delta) =
            remainder_before_suffix_runs(suffix, &after_prefix)
        else {
            continue;
        };
        if delta.is_empty() {
            continue;
        }

        return Some((
            GrowthPos::BeforeStableSuffix,
            suffix.to_vec(),
            delta,
        ));
    }

    None
}

fn suffix_after_prefix_runs(
    prefix: &[RunSig],
    whole: &[RunSig],
) -> Option<Vec<RunSig>> {
    let delta = remainder_after_prefix_runs(prefix, whole)?;
    if delta.is_empty() { None } else { Some(delta) }
}

fn prefix_before_suffix_runs(
    suffix: &[RunSig],
    whole: &[RunSig],
) -> Option<Vec<RunSig>> {
    let delta = remainder_before_suffix_runs(suffix, whole)?;
    if delta.is_empty() { None } else { Some(delta) }
}

/// Result of subtracting one compatible run count from another.
#[derive(Clone, Copy, PartialEq, Eq)]
enum RunCountRemainder {
    Consumed,
    Remaining(BlockCount),
}

/// Return how much of `whole` remains after removing `prefix`.
///
/// Exact runs may split exact runs. Lower-bounded runs may split another
/// lower-bounded run: `AtLeast(a)` plus an exact suffix of `b-a` cells is
/// exactly `AtLeast(b)`. Mixed exact/lower-bounded matching is deliberately
/// rejected so a periodic certificate never chooses a particular realization
/// of an indefinite run.
const fn subtract_run_count(
    prefix: BlockCount,
    whole: BlockCount,
) -> Option<RunCountRemainder> {
    match (prefix, whole) {
        (BlockCount::Exact(p), BlockCount::Exact(w))
        | (BlockCount::AtLeast(p), BlockCount::AtLeast(w)) => {
            if p > w {
                None
            } else if p == w {
                Some(RunCountRemainder::Consumed)
            } else {
                Some(RunCountRemainder::Remaining(BlockCount::Exact(
                    w - p,
                )))
            }
        },
        _ => None,
    }
}

/// Return the part of `whole` after removing `prefix` from the near/head end.
fn remainder_after_prefix_runs(
    prefix: &[RunSig],
    whole: &[RunSig],
) -> Option<Vec<RunSig>> {
    let mut wi = 0;

    for (pi, &p) in prefix.iter().enumerate() {
        let &w = whole.get(wi)?;
        if p.color != w.color {
            return None;
        }

        match subtract_run_count(p.count, w.count)? {
            RunCountRemainder::Consumed => wi += 1,
            RunCountRemainder::Remaining(count) => {
                if pi + 1 != prefix.len() {
                    return None;
                }

                let mut out = Vec::with_capacity(whole.len() - wi);
                out.push(RunSig {
                    color: w.color,
                    count,
                });
                out.extend_from_slice(&whole[wi + 1..]);
                return Some(normalize_run_sig_vec(out));
            },
        }
    }

    Some(whole[wi..].to_vec())
}

/// Return the part of `whole` before removing `suffix` from the far end.
fn remainder_before_suffix_runs(
    suffix: &[RunSig],
    whole: &[RunSig],
) -> Option<Vec<RunSig>> {
    let mut wi = whole.len();

    for (si, &s) in suffix.iter().rev().enumerate() {
        let &w = whole.get(wi.checked_sub(1)?)?;
        if s.color != w.color {
            return None;
        }

        match subtract_run_count(s.count, w.count)? {
            RunCountRemainder::Consumed => wi -= 1,
            RunCountRemainder::Remaining(count) => {
                if si + 1 != suffix.len() {
                    return None;
                }

                let mut out = whole[..wi - 1].to_vec();
                out.push(RunSig {
                    color: w.color,
                    count,
                });
                return Some(normalize_run_sig_vec(out));
            },
        }
    }

    Some(whole[..wi].to_vec())
}

const fn add_run_counts(
    left: BlockCount,
    right: BlockCount,
) -> BlockCount {
    let minimum = left.minimum() + right.minimum();
    if left.is_exact() && right.is_exact() {
        BlockCount::Exact(minimum)
    } else {
        BlockCount::AtLeast(minimum)
    }
}

fn normalize_run_sig_vec(runs: Vec<RunSig>) -> Vec<RunSig> {
    let mut out: Vec<RunSig> = Vec::with_capacity(runs.len());
    for r in runs {
        if let Some(last) = out.last_mut()
            && last.color == r.color
        {
            last.count = add_run_counts(last.count, r.count);
            continue;
        }
        out.push(r);
    }
    out
}

/**************************************/

// Certificate-only mixed-frontier coverage closer.
//
// The exact PeriodicHistory requires a bijection between F[t] and F[t+p].
// Some blank cones instead have growing frontiers where F[t+p] contains extra
// configs, but every extra config is still an instance of a repeated growth
// family already generated from F[t].  This detector allows those extras, but
// only if *each* config in the later frontier is covered by the same per-phase
// stable/growth relation for several samples.  It never deletes a branch and
// never widens a live config.
#[derive(Default)]
struct CoveragePeriodicHistory {
    snaps: Vec<PeriodicSnap>,
}

impl CoveragePeriodicHistory {
    const KEEP: usize = COVERAGE_PERIODIC_MAX_NEED * 2 + 8;
    const MAX_FRONTIER_FOR_CLOSER: usize = 50_000;

    fn clear(&mut self) {
        self.snaps.clear();
    }

    fn observe_frontier(
        &mut self,
        step: Steps,
        front: FastFrontier,
    ) -> Option<Steps> {
        if front.is_empty() {
            return None;
        }

        if front.len() > Self::MAX_FRONTIER_FOR_CLOSER {
            self.clear();
            return None;
        }

        self.snaps.push(PeriodicSnap { step, front });
        if self.snaps.len() > Self::KEEP {
            self.snaps.remove(0);
        }

        let cycle_start_idx = self.detect_any_phase_coverage()?;
        Some(self.snaps[cycle_start_idx].step)
    }

    fn detect_any_phase_coverage(&self) -> Option<usize> {
        for period in coverage_periodic_periods() {
            let need = coverage_periodic_need(period);
            if self.snaps.len() < need {
                continue;
            }

            let start = self.snaps.len() - need;
            if let Some(cycle_start) =
                self.detect_period_coverage_from(start, period)
            {
                return Some(cycle_start);
            }
        }
        None
    }

    fn detect_period_coverage_from(
        &self,
        start: usize,
        period: usize,
    ) -> Option<usize> {
        let expected =
            self.expected_coverage_signatures(start, period)?;
        let need = coverage_periodic_need(period);

        for j in start..start + need - period {
            let phase = (j - start) % period;
            if !frontier_covered_by_sigs(
                &self.snaps[j].front,
                &self.snaps[j + period].front,
                &expected[phase],
            ) {
                return None;
            }
        }

        let mut cycle_start = start;
        while cycle_start > 0 {
            let candidate = cycle_start - 1;
            if candidate + period >= self.snaps.len() {
                break;
            }

            let phase = (candidate + period - start) % period;
            if !frontier_covered_by_sigs(
                &self.snaps[candidate].front,
                &self.snaps[candidate + period].front,
                &expected[phase],
            ) {
                break;
            }

            cycle_start = candidate;
        }

        Some(cycle_start)
    }

    fn expected_coverage_signatures(
        &self,
        start: usize,
        period: usize,
    ) -> Option<Vec<Vec<CoverSig>>> {
        let need = coverage_periodic_need(period);
        let end = start + need;
        if end > self.snaps.len() {
            return None;
        }

        let mut expected: Vec<Option<Vec<CoverSig>>> =
            vec![None; period];
        let mut counts = vec![0; period];

        for j in start..end.saturating_sub(period) {
            let phase = (j - start) % period;
            match &expected[phase] {
                None => {
                    let sigs = coverage_signature_union(
                        &self.snaps[j].front,
                        &self.snaps[j + period].front,
                    )?;
                    if sigs.is_empty()
                        || !sigs.iter().any(CoverSig::is_growth)
                    {
                        return None;
                    }
                    expected[phase] = Some(sigs);
                },
                Some(sigs) => {
                    if !frontier_covered_by_sigs(
                        &self.snaps[j].front,
                        &self.snaps[j + period].front,
                        sigs,
                    ) {
                        return None;
                    }
                },
            }
            counts[phase] += 1;
        }

        if counts
            .iter()
            .any(|&count| count < COVERAGE_PERIODIC_MIN_PAIRS_PER_PHASE)
        {
            return None;
        }

        expected.into_iter().collect()
    }
}

const COVERAGE_PERIODIC_MIN_PERIOD: usize = 1;
const COVERAGE_PERIODIC_MAX_PERIOD: usize = 7;
const COVERAGE_PERIODIC_MIN_PAIRS_PER_PHASE: usize = 5;
const COVERAGE_PERIODIC_MAX_NEED: usize = COVERAGE_PERIODIC_MAX_PERIOD
    * (COVERAGE_PERIODIC_MIN_PAIRS_PER_PHASE + 1);

const fn coverage_periodic_periods() -> core::ops::RangeInclusive<usize>
{
    COVERAGE_PERIODIC_MIN_PERIOD..=COVERAGE_PERIODIC_MAX_PERIOD
}

const fn coverage_periodic_need(period: usize) -> usize {
    period * (COVERAGE_PERIODIC_MIN_PAIRS_PER_PHASE + 1)
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
enum CoverSig {
    Stable(FastCfg),
    Grow(BranchSig),
}

impl CoverSig {
    const fn is_growth(&self) -> bool {
        matches!(self, Self::Grow(_))
    }
}

fn coverage_signature_union(
    old: &[FastCfg],
    new: &[FastCfg],
) -> Option<Vec<CoverSig>> {
    if old.is_empty() || new.is_empty() {
        return None;
    }

    let mut out = Vec::new();
    for cb in new {
        let cands = cover_candidates_for_new(old, cb);
        if cands.is_empty() {
            return None;
        }
        out.extend(cands);
    }

    out.sort_unstable();
    out.dedup();
    Some(out)
}

fn frontier_covered_by_sigs(
    old: &[FastCfg],
    new: &[FastCfg],
    sigs: &[CoverSig],
) -> bool {
    if old.is_empty() || new.is_empty() || sigs.is_empty() {
        return false;
    }

    new.iter().all(|cb| {
        cover_candidates_for_new(old, cb)
            .into_iter()
            .any(|cand| sigs.binary_search(&cand).is_ok())
    })
}

fn cover_candidates_for_new(
    old: &[FastCfg],
    cb: &FastCfg,
) -> Vec<CoverSig> {
    let mut out = Vec::new();
    for ca in old {
        if ca == cb {
            out.push(CoverSig::Stable(ca.clone()));
        }
        for sig in growth_sigs_between(ca, cb) {
            out.push(CoverSig::Grow(sig));
        }
    }

    out.sort_unstable();
    out.dedup();
    out
}

fn growth_sigs_between(ca: &FastCfg, cb: &FastCfg) -> Vec<BranchSig> {
    if ca.state != cb.state || ca.scan != cb.scan {
        return Vec::new();
    }

    let mut out = Vec::new();

    if ca.r_end == cb.r_end
        && ca.right == cb.right
        && ca.l_end == cb.l_end
        && let Some((grow_pos, anchor, delta)) =
            left_growth_delta(&ca.left, &cb.left)
    {
        out.push(BranchSig {
            state: ca.state,
            scan: ca.scan,
            dh: cb.head - ca.head,
            grow_side: Side::Left,
            grow_pos,
            grow_end: ca.l_end,
            grow_anchor: anchor,
            grow_delta: delta,
            same_end: ca.r_end,
            same: ca.right.clone(),
        });
    }

    if ca.l_end == cb.l_end
        && ca.left == cb.left
        && ca.r_end == cb.r_end
        && let Some((grow_pos, anchor, delta)) =
            right_growth_delta(&ca.right, &cb.right)
    {
        out.push(BranchSig {
            state: ca.state,
            scan: ca.scan,
            dh: cb.head - ca.head,
            grow_side: Side::Right,
            grow_pos,
            grow_end: ca.r_end,
            grow_anchor: anchor,
            grow_delta: delta,
            same_end: ca.l_end,
            same: ca.left.clone(),
        });
    }

    out.sort_unstable();
    out.dedup();
    out
}

/**************************************/

struct AntichainEntry {
    hash: u64,
    tape: Tape,
}

/// Exact near-head class for one span. Subsumption can only hold when the
/// first explicit runs are compatible, except that an empty unknown span is a
/// wildcard prefix. Lower-bounded runs share a color bucket because their
/// minimums are ordered and are checked by exact subsumption afterwards.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum SpanBucketKey {
    EmptyUnknown,
    EmptyBlanks,
    RunExact(Color, Count),
    RunAtLeast(Color),
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct AntichainBucketKey {
    left: SpanBucketKey,
    right: SpanBucketKey,
}

fn span_bucket_key(span: &Span) -> SpanBucketKey {
    span.span.first().map_or_else(
        || match span.end {
            TapeEnd::Unknown => SpanBucketKey::EmptyUnknown,
            TapeEnd::Blanks => SpanBucketKey::EmptyBlanks,
        },
        |block| match block.count {
            BlockCount::Exact(count) => {
                SpanBucketKey::RunExact(block.color, count)
            },
            BlockCount::AtLeast(_) => {
                SpanBucketKey::RunAtLeast(block.color)
            },
        },
    )
}

fn antichain_bucket_key(tape: &Tape) -> AntichainBucketKey {
    AntichainBucketKey {
        left: span_bucket_key(&tape.lspan),
        right: span_bucket_key(&tape.rspan),
    }
}

/// Fill the bucket classes whose members may subsume `span`.
///
/// - Empty unknown is always a candidate.
/// - Empty blank covers only another empty blank span.
/// - Any nonempty run may be covered by a lower-bounded run of the same color.
/// - An exact run may additionally be covered by the identical exact bucket.
fn covering_span_bucket_keys(
    span: &Span,
) -> ([SpanBucketKey; 3], usize) {
    let mut keys = [SpanBucketKey::EmptyUnknown; 3];
    let mut len = 1;

    match span.span.first() {
        None if span.end == TapeEnd::Blanks => {
            keys[len] = SpanBucketKey::EmptyBlanks;
            len += 1;
        },
        Some(block) => {
            keys[len] = SpanBucketKey::RunAtLeast(block.color);
            len += 1;

            if let BlockCount::Exact(count) = block.count {
                keys[len] = SpanBucketKey::RunExact(block.color, count);
                len += 1;
            }
        },
        None => {},
    }

    (keys, len)
}

/// Cheap necessary condition for `candidate` to subsume a member of `bucket`.
/// Exact span comparison is still performed afterwards.
fn span_can_subsume_bucket(
    candidate: &Span,
    bucket: SpanBucketKey,
) -> bool {
    candidate.span.first().map_or_else(
        || match candidate.end {
            TapeEnd::Unknown => true,
            TapeEnd::Blanks => bucket == SpanBucketKey::EmptyBlanks,
        },
        |block| match block.count {
            BlockCount::AtLeast(_) => matches!(
                bucket,
                SpanBucketKey::RunExact(color, _)
                    | SpanBucketKey::RunAtLeast(color)
                    if color == block.color
            ),
            BlockCount::Exact(count) => {
                bucket == SpanBucketKey::RunExact(block.color, count)
            },
        },
    )
}

#[derive(Default)]
struct Antichain(Dict<AntichainBucketKey, Vec<AntichainEntry>>);

impl Antichain {
    fn insert(&mut self, tape: &Tape) -> bool {
        let hash = tape.hash();

        // An existing tape that covers the candidate must lie in one of at
        // most 3 x 3 exact near-head buckets.  This avoids scanning unrelated
        // first colors/counts in a large (state, scan, head) antichain.
        let (left_keys, left_len) =
            covering_span_bucket_keys(&tape.lspan);
        let (right_keys, right_len) =
            covering_span_bucket_keys(&tape.rspan);

        for &left in &left_keys[..left_len] {
            for &right in &right_keys[..right_len] {
                let key = AntichainBucketKey { left, right };
                let Some(entries) = self.0.get(&key) else {
                    continue;
                };

                for old in entries {
                    if old.hash == hash && old.tape == *tape {
                        return false;
                    }
                    if old.tape.subsumes(tape) {
                        return false;
                    }
                }
            }
        }

        // Remove entries covered by the candidate.  The bucket predicate is a
        // necessary condition, so exact subsumption remains the authority.
        // There are at most (colors/count classes + 2)^2 buckets, usually far
        // fewer than tapes; scanning bucket headers is much cheaper than
        // comparing every tape structurally.
        self.0.retain(|key, entries| {
            if !span_can_subsume_bucket(&tape.lspan, key.left)
                || !span_can_subsume_bucket(&tape.rspan, key.right)
            {
                return true;
            }

            let mut index = 0;
            while index < entries.len() {
                if tape.subsumes(&entries[index].tape) {
                    entries.swap_remove(index);
                } else {
                    index += 1;
                }
            }

            !entries.is_empty()
        });

        let key = antichain_bucket_key(tape);
        self.0.entry(key).or_default().push(AntichainEntry {
            hash,
            tape: tape.clone(),
        });
        true
    }
}

#[derive(Default)]
struct Antichains(Dict<(State, Color, Pos), Antichain>);

impl Antichains {
    fn insert(&mut self, cfg: &Config) -> bool {
        let key = (cfg.state, cfg.tape.scan, cfg.tape.head);

        self.0.entry(key).or_default().insert(&cfg.tape)
    }
}

/**************************************/

#[expect(clippy::multiple_inherent_impl)]
impl<const s: usize, const c: usize> Prog<s, c> {
    pub fn is_reversible(&self) -> bool {
        self.get_entrypoints().values().all(|(same, diff)| {
            let mut shift = None;
            let mut seen_print = [false; c];

            for &(_, (print, sh)) in same.iter().chain(diff) {
                match shift {
                    Some(prev) if prev != sh => return false,
                    Some(_) => {},
                    None => shift = Some(sh),
                }

                let print = print as usize;

                if seen_print[print] {
                    return false;
                }

                seen_print[print] = true;
            }

            true
        })
    }
}

#[test]
fn test_is_reversible() {
    assert!(Prog::<2, 2>::from("0RB ...  1LA 1RB").is_reversible());
    assert!(Prog::<2, 2>::from("0RB 0LA  1LA 1RB").is_reversible());
    assert!(!Prog::<2, 2>::from("0RB 1LA  1LA 1RB").is_reversible());

    assert!(
        Prog::<3, 2>::from("0RB ...  0LC 1RA  1RB 1LC").is_reversible()
    );
    assert!(
        Prog::<3, 2>::from("0RB 0RA  0LC 1RA  1RB 1LC").is_reversible()
    );
    assert!(
        !Prog::<3, 2>::from("0RB 0RB  0LC 1RA  1RB 1LC")
            .is_reversible()
    );

    assert!(
        Prog::<4, 2>::from("1RB 0LD  0LC 0RB  1LA 1LD  1LC ...")
            .is_reversible()
    );
    assert!(
        Prog::<4, 2>::from("1RB 0LD  0LC 0RB  1LA 1LD  1LC 0LA")
            .is_reversible()
    );
    assert!(
        !Prog::<4, 2>::from("1RB 0LD  0LC 0RB  1LA 1LD  1LC 1LA")
            .is_reversible()
    );

    assert!(
        Prog::<5, 2>::from(
            "1RB 0RD  1RC 0RB  1RD ...  1LE 1LA  0LE 0LA"
        )
        .is_reversible()
    );
    assert!(
        Prog::<5, 2>::from(
            "1RB 0RD  1RC 0RB  1RD 0RC  1LE 1LA  0LE 0LA"
        )
        .is_reversible()
    );
    assert!(
        !Prog::<5, 2>::from(
            "1RB 0RD  1RC 0RB  1RD 1RC  1LE 1LA  0LE 0LA"
        )
        .is_reversible()
    );

    assert!(
        Prog::<6, 2>::from(
            "1RB 1LD  1LC 1RE  0LD 0LC  0RE 0RF  0RA ...  1RF 1RA"
        )
        .is_reversible()
    );
    assert!(
        Prog::<6, 2>::from(
            "1RB 1LD  1LC 1RE  0LD 0LC  0RE 0RF  0RA 0RB  1RF 1RA"
        )
        .is_reversible()
    );
    assert!(
        !Prog::<6, 2>::from(
            "1RB 1LD  1LC 1RE  0LD 0LC  0RE 0RF  0RA 1RB  1RF 1RA"
        )
        .is_reversible()
    );

    assert!(Prog::<7, 2>::from("1RB 1LD  0LC 0LD  1LC 1LA  0LA 1RE  0RF 0RE  0RG 1RF  0RB ...").is_reversible());
    assert!(Prog::<7, 2>::from("1RB 1LD  0LC 0LD  1LC 1LA  0LA 1RE  0RF 0RE  0RG 1RF  0RB 1RG").is_reversible());
    assert!(!Prog::<7, 2>::from("1RB 1LD  0LC 0LD  1LC 1LA  0LA 1RE  0RF 0RE  0RG 1RF  0RB 1LG").is_reversible());
}
