use core::{
    array,
    fmt::{self, Display, Formatter},
    iter::once,
};

use ahash::{AHashMap as Dict, AHashSet as Set};

use crate::{
    Color, Goal, Instr, Prog, Shift, Slot, State,
    instrs::show_state,
    tape::{Block as _, LilBlock as Block, LilCount as Count},
};

use Goal::*;

pub type Segments = usize;

const MAX_DEPTH: usize = 3_000;

/**************************************/

pub enum SegmentResult {
    Halt,
    Blank,
    Repeat,
    Spinout,
    DepthLimit,
    SegmentLimit,
    Refuted(Segments),
}

impl SegmentResult {
    pub const fn is_refuted(&self) -> bool {
        matches!(self, Self::Refuted(_))
    }
}

/**************************************/

impl<const s: usize, const c: usize> Prog<s, c> {
    pub fn seg_cant_halt(&self, segs: Segments) -> SegmentResult {
        segment_cant_reach(self, segs, Halt)
    }

    pub fn seg_cant_blank(&self, segs: Segments) -> SegmentResult {
        segment_cant_reach(self, segs, Blank)
    }

    pub fn seg_cant_spin_out(&self, segs: Segments) -> SegmentResult {
        segment_cant_reach(self, segs, Spinout)
    }
}

/**************************************/

enum SearchResult {
    Limit,
    Repeat,
    Reached,
    Found(Goal),
}

use SearchResult::*;

impl From<Goal> for SegmentResult {
    fn from(goal: Goal) -> Self {
        match goal {
            Halt => Self::Halt,
            Blank => Self::Blank,
            Spinout => Self::Spinout,
        }
    }
}

fn segment_cant_reach<const s: usize, const c: usize>(
    prog: &Prog<s, c>,
    segs: Segments,
    goal: Goal,
) -> SegmentResult {
    assert!(segs >= 2);

    let prog = AnalyzedProg::new(prog);

    if (goal.is_halt() && prog.halts.is_empty())
        || (goal.is_spinout() && prog.spinouts.is_empty())
    {
        return SegmentResult::Refuted(0);
    }

    for seg in 2..=segs {
        let Some(result) = all_segments_reached(&prog, 2 + seg, goal)
        else {
            return SegmentResult::Refuted(seg);
        };

        match result {
            Limit => return SegmentResult::DepthLimit,
            Repeat => return SegmentResult::Repeat,
            Found(found) => return found.into(),
            Reached => {},
        }
    }

    SegmentResult::SegmentLimit
}

/**************************************/

fn all_segments_reached<const s: usize, const c: usize>(
    prog: &AnalyzedProg<s, c>,
    seg: Segments,
    goal: Goal,
) -> Option<SearchResult> {
    let mut configs =
        Configs::<s>::new(&prog.halts, &prog.spinouts, seg, goal);

    let branches = &prog.branches;

    while let Some(mut config) = configs.next() {
        if let Some(result) =
            prog.prog.run_to_edge(&mut config, goal, &mut configs)
        {
            match result {
                Repeat if config.init => {
                    return Some(
                        if goal.is_blank() && config.tape.blank() {
                            Found(Blank)
                        } else {
                            Repeat
                        },
                    );
                },

                Found(Halt) => {
                    if config.init {
                        return Some(Found(Halt));
                    }

                    if goal.is_halt()
                        && configs.check_reached(&config, goal)
                    {
                        return Some(Reached);
                    }

                    continue;
                },

                Found(Blank) => {
                    assert!(goal.is_blank());

                    if configs.check_reached(&config, goal) {
                        return Some(Reached);
                    }

                    continue;
                },

                Found(Spinout) => {
                    if config.init {
                        return Some(Found(Spinout));
                    }

                    assert!(goal.is_spinout());

                    if configs.check_reached(&config, goal) {
                        return Some(Reached);
                    }

                    continue;
                },

                _ => {
                    continue;
                },
            }
        }

        let goal_tape = match goal {
            Halt => true,
            Blank => config.tape.blank(),
            Spinout => {
                if let Some(&shift) = prog.spinouts.get(&config.state) {
                    shift == config.tape.side() || config.tape.blank()
                } else {
                    false
                }
            },
        };

        if goal_tape && configs.check_reached(&config, goal) {
            return Some(Reached);
        }

        let (diffs, dirs) = &branches[config.state as usize];

        let blank = config.tape.blank();

        configs.branch_in(&config.tape, dirs, blank);

        configs.branch_out(config, diffs, blank);

        if configs.check_depth() {
            return Some(Limit);
        }
    }

    None
}

/**************************************/

type Pos = usize;

struct Configs<const S: usize> {
    seg: Segments,

    todo: Vec<Config>,
    tape_ids: Dict<Tape, u32>,
    next_tape_id: u32,
    blanks: [Vec<bool>; S],
    goal_states: [bool; S],
    reached: [Vec<bool>; S],
    reached_counts: [usize; S],
    blank_union: Vec<bool>,
    blank_union_count: usize,
    seen_bits: [Vec<u64>; S],
    seen_counts: [usize; S],
    max_seen_len: usize,
    next_init_pos: Pos,
}

impl<const S: usize> Configs<S> {
    fn new(
        halts: &Halts,
        spinouts: &Spinouts,
        seg: Segments,
        goal: Goal,
    ) -> Self {
        let mut goal_states = [false; S];

        match goal {
            Blank => {},
            Halt => {
                for &state in halts {
                    goal_states[state as usize] = true;
                }
            },
            Spinout => {
                for (&state, _) in spinouts {
                    goal_states[state as usize] = true;
                }
            },
        }

        Self {
            seg,
            todo: vec![],
            tape_ids: Dict::with_capacity(MAX_DEPTH * S),
            next_tape_id: 0,
            blanks: array::from_fn(|_| vec![false; seg]),
            goal_states,
            reached: array::from_fn(|_| vec![false; seg]),
            reached_counts: [0; S],
            blank_union: vec![false; seg],
            blank_union_count: 0,
            seen_bits: array::from_fn(|_| Vec::new()),
            seen_counts: [0; S],
            max_seen_len: 0,
            next_init_pos: 0,
        }
    }

    fn add_todo(&mut self, config: Config) {
        self.todo.push(config);
    }

    const fn check_depth(&self) -> bool {
        MAX_DEPTH < self.max_seen_len
    }

    fn next_init(&mut self) -> Option<Config> {
        let blanks0 = &mut self.blanks[0];

        while self.next_init_pos < self.seg
            && blanks0[self.next_init_pos]
        {
            self.next_init_pos += 1;
        }

        if self.next_init_pos >= self.seg {
            return None;
        }

        let pos = self.next_init_pos;
        self.next_init_pos += 1;

        blanks0[pos] = true;

        if !self.blank_union[pos] {
            self.blank_union[pos] = true;
            self.blank_union_count += 1;
        }

        Some(Config::init(self.seg, pos))
    }

    fn tape_id(&mut self, tape: &Tape) -> u32 {
        if let Some(&id) = self.tape_ids.get(tape) {
            id
        } else {
            let id = self.next_tape_id;
            self.next_tape_id += 1;
            self.tape_ids.insert(tape.clone(), id);
            id
        }
    }

    fn check_seen_nonblank_id(
        &mut self,
        state: State,
        id: u32,
    ) -> Option<bool> {
        let idx = state as usize;
        let bits = &mut self.seen_bits[idx];

        let bit = id as usize;
        let word_idx = bit >> 6;
        let mask = 1_u64 << (bit & 63);

        if word_idx >= bits.len() {
            bits.resize(word_idx + 1, 0);
        }

        let word = &mut bits[word_idx];

        if (*word & mask) != 0 {
            return None;
        }

        *word |= mask;

        let counts = &mut self.seen_counts[idx];
        *counts += 1;

        if self.max_seen_len < *counts {
            self.max_seen_len = *counts;
        }

        Some(false)
    }

    fn check_seen(
        &mut self,
        state: State,
        tape: &Tape,
        blank: bool,
    ) -> Option<bool> {
        if blank {
            let blanks = &mut self.blanks[state as usize];

            let pos = tape.pos();

            if blanks[pos] {
                return None;
            }

            blanks[pos] = true;

            if !self.blank_union[pos] {
                self.blank_union[pos] = true;
                self.blank_union_count += 1;
            }

            return Some(state == 0);
        }

        let id = self.tape_id(tape);

        self.check_seen_nonblank_id(state, id)
    }

    fn check_reached(&mut self, config: &Config, goal: Goal) -> bool {
        if goal.is_blank() {
            return self.check_reached_blank(config);
        }

        let idx = config.state as usize;

        if !self.goal_states[idx] {
            return false;
        }

        let reached = &mut self.reached[idx];

        let pos = config.tape.pos();

        if !reached[pos] {
            reached[pos] = true;
            self.reached_counts[idx] += 1;
        }

        self.reached_counts[idx] == self.seg
    }

    fn check_reached_blank(&mut self, config: &Config) -> bool {
        let blanks = &mut self.blanks[config.state as usize];

        let pos = config.tape.pos();

        if !blanks[pos] {
            blanks[pos] = true;

            if !self.blank_union[pos] {
                self.blank_union[pos] = true;
                self.blank_union_count += 1;
            }
        }

        self.blank_union_count == self.seg
    }

    fn branch_in(&mut self, tape: &Tape, dirs: &Dirs, blank: bool) {
        let shift = !tape.side();

        let states = &dirs[usize::from(shift)];

        if states.is_empty() {
            return;
        }

        let mut stepped = tape.clone();

        stepped.step_in(shift);

        let Some((&last_state, rest)) = states.split_last() else {
            return;
        };

        if blank {
            for &state in rest {
                let Some(init) = self.check_seen(state, &stepped, true)
                else {
                    continue;
                };

                let config = Config::new(state, stepped.clone(), init);

                self.add_todo(config);
            }

            if let Some(init) =
                self.check_seen(last_state, &stepped, true)
            {
                let config = Config::new(last_state, stepped, init);

                self.add_todo(config);
            }
        } else {
            let id = self.tape_id(&stepped);

            for &state in rest {
                let Some(init) = self.check_seen_nonblank_id(state, id)
                else {
                    continue;
                };

                let config = Config::new(state, stepped.clone(), init);

                self.add_todo(config);
            }

            if let Some(init) =
                self.check_seen_nonblank_id(last_state, id)
            {
                let config = Config::new(last_state, stepped, init);

                self.add_todo(config);
            }
        }
    }

    fn branch_out(
        &mut self,
        mut config: Config,
        diffs: &Diffs,
        blank: bool,
    ) {
        let tape = &config.tape;

        let Some((last_next, diffs)) = diffs.split_last() else {
            return;
        };

        if blank {
            for &state in diffs {
                let Some(init) = self.check_seen(state, tape, true)
                else {
                    continue;
                };

                let next_tape = tape.clone();

                let todo = Config::new(state, next_tape, init);

                self.add_todo(todo);
            }

            if let Some(init) = self.check_seen(*last_next, tape, true)
            {
                config.state = *last_next;

                config.init = init;

                self.add_todo(config);
            }
        } else {
            let id = self.tape_id(tape);

            for &state in diffs {
                let Some(init) = self.check_seen_nonblank_id(state, id)
                else {
                    continue;
                };

                let next_tape = tape.clone();

                let todo = Config::new(state, next_tape, init);

                self.add_todo(todo);
            }

            if let Some(init) =
                self.check_seen_nonblank_id(*last_next, id)
            {
                config.state = *last_next;

                config.init = init;

                self.add_todo(config);
            }
        }
    }
}

impl<const S: usize> Iterator for Configs<S> {
    type Item = Config;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_init().or_else(|| self.todo.pop())
    }
}

/**************************************/

#[derive(PartialEq, Eq, Hash, Clone)]
struct Config {
    state: State,
    tape: Tape,

    init: bool,
}

impl Config {
    const fn new(state: State, tape: Tape, init: bool) -> Self {
        Self { state, tape, init }
    }

    fn init(seg: Segments, pos: Pos) -> Self {
        Self::new(0, Tape::init(seg, pos), true)
    }

    fn slot(&self) -> Option<Slot> {
        Some((self.state, self.tape.scan?))
    }

    fn step(&mut self, &(print, shift, state): &Instr) {
        self.tape.step(shift, print, state == self.state);
        self.state = state;
    }

    fn spinout(&self, &(_, shift, state): &Instr) -> bool {
        self.state == state && self.tape.at_edge(shift)
    }
}

impl<const s: usize, const c: usize> Prog<s, c> {
    #[expect(clippy::unwrap_in_result)]
    fn run_to_edge(
        &self,
        config: &mut Config,
        goal: Goal,
        configs: &mut Configs<s>,
    ) -> Option<SearchResult> {
        config.tape.scan?;

        let mut step = false;
        let mut copy = config.clone();

        while let Some(slot) = config.slot() {
            let Some(instr @ &(print, _, state)) = self.get(&slot)
            else {
                return Some(Found(Halt));
            };

            if (config.init || goal.is_spinout())
                && config.spinout(instr)
                && (config.init || configs.check_reached(config, goal))
            {
                return Some(Found(Spinout));
            }

            config.step(instr);

            if print == 0 && config.tape.blank() {
                if state == 0 {
                    if config.init {
                        return Some(Repeat);
                    }

                    config.init = true;
                }

                let pos = config.tape.pos();

                let blanks = &mut configs.blanks[state as usize];

                if !blanks[pos] {
                    blanks[pos] = true;

                    if !configs.blank_union[pos] {
                        configs.blank_union[pos] = true;
                        configs.blank_union_count += 1;
                    }
                }

                if goal.is_blank() {
                    return Some(Found(Blank));
                }
            }

            if !step {
                step = true;
                continue;
            }

            copy.step(self.get(&copy.slot().unwrap()).unwrap());

            if copy.state == config.state && copy.tape == config.tape {
                return Some(Repeat);
            }

            step = false;
        }

        None
    }
}

impl Display for Config {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{} | {}", show_state(Some(self.state)), self.tape)
    }
}

/**************************************/

#[derive(PartialEq, Eq, Hash, Clone)]
struct Span {
    blocks: Vec<Block>,
    len: Pos,
}

impl Span {
    fn new(blocks: Vec<Block>) -> Self {
        let len = blocks.iter().map(|block| block.count as Pos).sum();
        Self { blocks, len }
    }

    fn blank(&self) -> bool {
        self.blocks.iter().all(Block::blank)
    }

    const fn len(&self) -> usize {
        self.len
    }

    const fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    fn push_block(&mut self, color: Color, count: Count) {
        self.blocks.push(Block::new(color, count));
        self.len += count as Pos;
    }

    fn push(&mut self, print: Color, stepped: Count) {
        if let Some(block) = self.blocks.last_mut()
            && block.color == print
        {
            block.add_count(stepped);
            self.len += stepped as Pos;
            return;
        }

        self.push_block(print, stepped);
    }

    fn pull(
        &mut self,
        scan: Color,
        skip: bool,
    ) -> (Option<Color>, Count) {
        let mut stepped: Count = 1;
        let mut removed: Pos = 0;

        if skip
            && !self.is_empty()
            && self.blocks.last().unwrap().get_color() == scan
        {
            let block = self.blocks.pop().unwrap();
            let count = *block.get_count() as Pos;
            removed += count;
            stepped += block.get_count();
        }

        if self.is_empty() {
            self.len -= removed;
            return (None, stepped);
        }

        let next_pull = self.blocks.last_mut().unwrap();

        let pull_color = next_pull.get_color();

        removed += 1;

        if next_pull.is_single() {
            self.blocks.pop();
        } else {
            next_pull.decrement();
        }

        self.len -= removed;

        (Some(pull_color), stepped)
    }

    fn take(&mut self) -> Color {
        assert!(!self.is_empty());

        let block = self.blocks.last_mut().unwrap();

        let color = block.color;

        if block.count == 1 {
            self.blocks.pop();
        } else {
            block.decrement();
        }

        self.len -= 1;

        color
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
struct Tape {
    scan: Option<Color>,
    lspan: Span,
    rspan: Span,
}

impl Tape {
    #[expect(clippy::cast_possible_truncation)]
    fn init(seg: Segments, pos: Pos) -> Self {
        assert!(seg >= 4);
        assert!(pos <= seg);

        let seg = seg as Count;
        let pos = pos as Count;
        let cells = seg - 2;

        let (scan, lspan, rspan) = if pos == 0 {
            (None, vec![], vec![Block::new(0, cells)])
        } else if pos == seg - 1 {
            (None, vec![Block::new(0, cells)], vec![])
        } else {
            let mut lspan = vec![];
            let mut rspan = vec![];

            let l_count = pos - 1;

            if l_count > 0 {
                lspan.push(Block::new(0, l_count));
            }

            let r_count = cells - pos;

            if r_count > 0 {
                rspan.push(Block::new(0, r_count));
            }

            (Some(0), lspan, rspan)
        };

        Self {
            scan,
            lspan: Span::new(lspan),
            rspan: Span::new(rspan),
        }
    }

    fn blank(&self) -> bool {
        matches!(self.scan, Some(0) | None)
            && self.lspan.blank()
            && self.rspan.blank()
    }

    fn at_edge(&self, edge: Shift) -> bool {
        self.scan == Some(0)
            && (if edge { &self.rspan } else { &self.lspan }).blank()
    }

    fn pos(&self) -> Pos {
        let l_len = self.lspan.len();

        l_len + Pos::from(self.scan.is_some() || 0 < l_len)
    }

    fn assert_edge(&self) {
        assert!(self.scan.is_none());
    }

    fn side(&self) -> Shift {
        self.assert_edge();

        self.rspan.is_empty()
    }

    fn step_in(&mut self, shift: Shift) {
        assert_eq!(self.side(), !shift);

        let pull = if shift {
            &mut self.rspan
        } else {
            &mut self.lspan
        };

        self.scan = Some(pull.take());
    }

    fn step(&mut self, shift: Shift, print: Color, skip: bool) {
        let (pull, push) = if shift {
            (&mut self.rspan, &mut self.lspan)
        } else {
            (&mut self.lspan, &mut self.rspan)
        };

        let (next_scan, stepped) = pull.pull(self.scan.unwrap(), skip);

        push.push(print, stepped);

        self.scan = next_scan;
    }

    #[cfg(test)]
    #[track_caller]
    fn from_spans(
        scan: Color,
        lblocks: Vec<Block>,
        rblocks: Vec<Block>,
    ) -> Self {
        Self {
            scan: Some(scan),
            lspan: Span::new(lblocks),
            rspan: Span::new(rblocks),
        }
    }
}

impl Display for Tape {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            self.lspan
                .blocks
                .iter()
                .map(ToString::to_string)
                .chain(once(format!(
                    "[{}]",
                    self.scan.map_or_else(
                        || "-".to_owned(),
                        |scan| scan.to_string()
                    )
                )))
                .chain(
                    self.rspan
                        .blocks
                        .iter()
                        .rev()
                        .map(ToString::to_string)
                )
                .collect::<Vec<_>>()
                .join(" ")
        )
    }
}

/**************************************/

type Halts = Set<State>;
type Spinouts = Dict<State, Shift>;

type Diffs = Vec<State>;
type Dirs = [Vec<State>; 2];

struct AnalyzedProg<'p, const s: usize, const c: usize> {
    prog: &'p Prog<s, c>,
    halts: Halts,
    spinouts: Spinouts,
    branches: [(Diffs, Dirs); s],
}

impl<'p, const states: usize, const colors: usize>
    AnalyzedProg<'p, states, colors>
{
    fn new(prog: &'p Prog<states, colors>) -> Self {
        let mut halts = Set::new();
        let mut spinouts = Dict::new();

        let mut branches: [(Diffs, Dirs); states] =
            array::from_fn(|_| (Vec::new(), [Vec::new(), Vec::new()]));

        #[expect(clippy::needless_range_loop)]
        for state_idx in 0..states {
            #[expect(clippy::cast_possible_truncation)]
            let state = state_idx as State;

            let mut diff = Set::new();
            let mut lefts = Set::new();
            let mut rights = Set::new();

            for color in 0..colors {
                #[expect(clippy::cast_possible_truncation)]
                let color = color as Color;

                let Some(&(_, shift, next)) = prog.get(&(state, color))
                else {
                    halts.insert(state);
                    continue;
                };

                if next == state {
                    if color == 0 {
                        spinouts.insert(next, shift);
                    }
                } else {
                    diff.insert(next);
                }

                (if shift { &mut rights } else { &mut lefts })
                    .insert(next);
            }

            let mut diff: Vec<_> = diff.into_iter().collect();
            diff.sort_unstable();

            let mut lefts: Vec<_> = lefts.into_iter().collect();
            lefts.sort_unstable();

            let mut rights: Vec<_> = rights.into_iter().collect();
            rights.sort_unstable();

            branches[state_idx] = (diff, [lefts, rights]);
        }

        Self {
            prog,
            halts,
            spinouts,
            branches,
        }
    }
}

/**************************************/

#[cfg(test)]
impl Tape {
    #[track_caller]
    fn scan(&self) -> Option<Color> {
        if self.scan.is_none() {
            assert!(self.lspan.is_empty() || self.rspan.is_empty());
        }

        self.scan
    }

    #[track_caller]
    fn assert(&self, expected: &str) {
        assert_eq!(self.to_string(), expected);
    }
}

#[cfg(test)]
impl Config {
    #[track_caller]
    fn assert(&self, expected: &str) {
        assert_eq!(self.to_string(), expected);
    }
}

#[test]
fn test_init() {
    let configs = [
        "A | [-] 0^5",
        "A | [0] 0^4",
        "A | 0 [0] 0^3",
        "A | 0^2 [0] 0^2",
        "A | 0^3 [0] 0",
        "A | 0^4 [0]",
        "A | 0^5 [-]",
    ];

    for (pos, config_str) in configs.iter().enumerate() {
        let config = Config::init(7, pos);

        config.assert(config_str);

        assert_eq!(config.tape.pos(), pos);
    }
}

#[test]
fn test_step_in() {
    let mut config = Config::init(7, 6);

    config.assert("A | 0^5 [-]");

    config.tape.step_in(false);

    config.assert("A | 0^4 [0]");
}

#[test]
fn test_seg_tape() {
    let prog = Prog::<3, 2>::from("1RB 1RC  0LA 0RA  0LB ...");

    let mut config = Config::init(4, 1);

    config.assert("A | [0] 0");

    let mut seen: Set<Config> = Set::new();

    while let Some(scan) = config.tape.scan() {
        seen.insert(config.clone());

        config.step(prog.get(&(config.state, scan)).unwrap());
    }

    assert!(config.tape.side());

    config.assert("B | 0 1 [-]");

    assert_eq!(seen.len(), 6);
}

#[cfg(test)]
macro_rules! assert_reached_states {
    ( $( ($prog:literal, ($s:literal, $c:literal)) => ( [$($halt:expr),* $(,)?], [$($spinout:expr),* $(,)?] ) ),* $(,)? ) => {
        $(
            {
                let comp = Prog::<$s, $c>::from($prog);
                let anal = AnalyzedProg::<$s, $c>::new(&comp);

                assert_eq!(
                    anal.halts,
                    [$($halt),*].into_iter().collect::<Set<_>>(),
                );

                assert_eq!(
                    anal.spinouts.keys().copied().collect::<Set<_>>(),
                    [$($spinout),*].into_iter().collect::<Set<_>>(),
                );
            }
        )*
    };
}

#[test]
fn test_reached_states() {
    assert_reached_states!(
        ("1RB 1RC  0LA 0RA  0LB ...", (3, 2)) => ([2], []),
        ("1RB ...  1LB 0RC  1LC 1LA", (3, 2)) => ([0], [1, 2]),
        ("1RB ... ...  2LB 1RB 1LB", (2, 3)) => ([0], [1]),
        ("1RB 0RB ...  2LA ... 0LB", (2, 3)) => ([0, 1], []),
        ("1RB ... 0RB ...  2LB 3RA 0RA 0RA", (2, 4)) => ([0], [1]),
    );
}

#[cfg(test)]
macro_rules! tape {
    (
        $ scan : literal,
        [ $ ( ( $ lcolor : literal, $ lcount: literal ) ), * ],
        [ $ ( ( $ rcolor : literal, $ rcount: literal ) ), * ]
    ) => {
        Tape::from_spans(
            $scan,
            vec![ $( Block::new( $lcolor, $lcount ) ),* ],
            vec![ $( Block::new( $rcolor, $rcount ) ),* ],
        )
    };
}

#[test]
fn test_step_edge() {
    let mut tape = tape! { 0, [], [(1, 1)] };

    tape.step(false, 1, false);

    tape.assert("[-] 1^2");
}

#[test]
fn test_skip() {
    let mut tape1 = tape! { 1, [(1, 20)], [] };
    tape1.assert("1^20 [1]");
    tape1.step(false, 0, true);
    tape1.assert("[-] 0^21");

    let mut tape2 = tape! { 1, [(1, 20)], [] };
    tape2.assert("1^20 [1]");
    tape2.step(false, 1, true);
    tape2.assert("[-] 1^21");

    let mut tape3 = tape! { 0, [(0, 1)], [] };
    tape3.assert("0 [0]");
    tape3.step(false, 1, true);
    tape3.assert("[-] 1^2");
}
