use core::{
    fmt::{Display, Formatter, Result},
    iter::once,
};

use std::collections::{BTreeMap as Dict, HashSet as Set};

use crate::{
    instrs::{
        show_state, Color, CompProg, Instr, Params, Shift, Slot, State,
    },
    tape::{BasicBlock as Block, Block as _, Count},
};

type Segments = usize;

const MAX_DEPTH: usize = 3_000;

/**************************************/

#[derive(PartialEq, Eq)]
enum Goal {
    Halt,
    Blank,
    Spinout,
}

enum SearchResult {
    Limit,
    Repeat,
    Reached,
    Found(Goal),
}

use Goal::*;
use SearchResult::*;

pub fn segment_cant_halt(
    prog: &CompProg,
    params: Params,
    segs: Segments,
) -> Option<Segments> {
    segment_cant_reach(prog, params, segs, &Halt)
}

pub fn segment_cant_blank(
    prog: &CompProg,
    params: Params,
    segs: Segments,
) -> Option<Segments> {
    segment_cant_reach(prog, params, segs, &Blank)
}

pub fn segment_cant_spin_out(
    prog: &CompProg,
    params: Params,
    segs: Segments,
) -> Option<Segments> {
    segment_cant_reach(prog, params, segs, &Spinout)
}

fn segment_cant_reach(
    prog: &CompProg,
    params: Params,
    segs: Segments,
    goal: &Goal,
) -> Option<Segments> {
    assert!(segs >= 2);

    let prog = AnalyzedProg::new(prog, params);

    if (goal == &Halt && prog.halts.is_empty())
        || (goal == &Spinout && prog.spinouts.is_empty())
    {
        return Some(0);
    }

    for seg in 2..=segs {
        let Some(result) = all_segments_reached(&prog, 2 + seg, goal)
        else {
            return Some(seg);
        };

        match result {
            Reached => continue,
            Limit => return None,
            Repeat => return Some(seg),
            Found(found) => return (found != *goal).then_some(seg),
        }
    }

    None
}

/**************************************/

fn all_segments_reached(
    prog: &AnalyzedProg,
    seg: Segments,
    goal: &Goal,
) -> Option<SearchResult> {
    let mut configs = Configs::new(prog, seg, goal);

    let branches = &prog.branches;

    #[cfg(debug_assertions)]
    println!();

    while let Some(mut config) = configs.next() {
        #[cfg(debug_assertions)]
        println!("{config}");

        if let Some(result) =
            config.run_to_edge(prog.prog, goal, &mut configs)
        {
            match result {
                Repeat if config.init => {
                    return Some(
                        if goal == &Blank && config.tape.blank() {
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

                    if goal == &Halt
                        && configs.check_reached(&config, goal)
                    {
                        return Some(Reached);
                    }

                    continue;
                },

                Found(Blank) => {
                    assert!(goal == &Blank);

                    if configs.check_reached(&config, goal) {
                        return Some(Reached);
                    }

                    continue;
                },

                Found(Spinout) => {
                    if config.init {
                        return Some(Found(Spinout));
                    }

                    assert!(goal == &Spinout);

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

        let (diffs, dirs) = &branches[&config.state];

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

struct Configs {
    seg: Segments,

    todo: Vec<Config>,
    seen: Dict<State, Set<Tape>>,
    blanks: Dict<State, Set<Pos>>,
    reached: Dict<State, Set<Pos>>,
}

impl Configs {
    fn new(prog: &AnalyzedProg, seg: Segments, goal: &Goal) -> Self {
        let reached = match goal {
            Blank => Dict::new(),
            Halt => prog
                .halts
                .iter()
                .map(|&state| (state, Set::new()))
                .collect(),
            Spinout => prog
                .spinouts
                .keys()
                .map(|&state| (state, Set::new()))
                .collect(),
        };

        Self {
            seg,
            todo: vec![],
            seen: Dict::new(),
            blanks: Dict::new(),
            reached,
        }
    }

    fn add_todo(&mut self, config: Config) {
        self.todo.push(config);
    }

    fn check_depth(&self) -> bool {
        self.seen.values().any(|seen| seen.len() > MAX_DEPTH)
    }

    fn next_init(&mut self) -> Option<Config> {
        let blanks = self.blanks.entry(0).or_default();

        let pos = (0..self.seg).find(|pos| !blanks.contains(pos))?;

        blanks.insert(pos);

        Some(Config::init(self.seg, pos))
    }

    fn check_seen(
        &mut self,
        state: State,
        tape: &Tape,
        blank: bool,
    ) -> Option<bool> {
        if blank {
            let blanks = self.blanks.entry(state).or_default();

            let pos = tape.pos();

            if blanks.contains(&pos) {
                return None;
            }

            blanks.insert(pos);
        } else {
            let seen = self.seen.entry(state).or_default();

            if seen.contains(tape) {
                return None;
            }

            seen.insert(tape.clone());
        }

        Some(blank && state == 0)
    }

    fn check_reached(&mut self, config: &Config, goal: &Goal) -> bool {
        if goal == &Blank {
            return self.check_reached_blank(config);
        }

        let Some(reached) = self.reached.get_mut(&config.state) else {
            return false;
        };

        reached.insert(config.tape.pos());

        reached.len() == self.seg
    }

    fn check_reached_blank(&mut self, config: &Config) -> bool {
        let Some(blanks) = self.blanks.get_mut(&config.state) else {
            return false;
        };

        blanks.insert(config.tape.pos());

        self.blanks
            .values()
            .flat_map(|set| set.iter())
            .collect::<Set<_>>()
            .len()
            == self.seg
    }

    fn branch_in(&mut self, tape: &Tape, dirs: &Dirs, blank: bool) {
        let shift = !tape.side();

        for &state in &dirs[&shift] {
            let mut next_tape = tape.clone();

            next_tape.step_in(shift);

            let Some(init) = self.check_seen(state, &next_tape, blank)
            else {
                continue;
            };

            let config = Config::new(state, next_tape, init);

            self.add_todo(config);
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

        for &state in diffs {
            let Some(init) = self.check_seen(state, tape, blank) else {
                continue;
            };

            let next_tape = tape.clone();

            let config = Config::new(state, next_tape, init);

            self.add_todo(config);
        }

        if let Some(init) = self.check_seen(*last_next, tape, blank) {
            config.state = *last_next;

            config.init = init;

            self.add_todo(config);
        }
    }
}

impl Iterator for Configs {
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
    #[expect(clippy::missing_const_for_fn)]
    fn new(state: State, tape: Tape, init: bool) -> Self {
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

    #[expect(clippy::unwrap_in_result)]
    fn run_to_edge(
        &mut self,
        prog: &CompProg,
        goal: &Goal,
        configs: &mut Configs,
    ) -> Option<SearchResult> {
        self.tape.scan?;

        let mut step = false;
        let mut copy = self.clone();

        while let Some(slot) = self.slot() {
            let Some(instr) = prog.get(&slot) else {
                return Some(Found(Halt));
            };

            if (self.init || goal == &Spinout)
                && self.spinout(instr)
                && (self.init || configs.check_reached(self, goal))
            {
                return Some(Found(Spinout));
            }

            self.step(instr);

            #[cfg(debug_assertions)]
            println!("    {self}");

            let &(print, _, state) = instr;

            if print == 0 && self.tape.blank() {
                if state == 0 {
                    if self.init {
                        return Some(Repeat);
                    }

                    self.init = true;
                }

                configs
                    .blanks
                    .entry(state)
                    .or_default()
                    .insert(self.tape.pos());

                if goal == &Blank {
                    return Some(Found(Blank));
                }
            }

            if !step {
                step = true;
                continue;
            }

            let instr = prog.get(&copy.slot().unwrap()).unwrap();

            copy.step(instr);

            if copy.state == self.state && copy.tape == self.tape {
                return Some(Repeat);
            }

            step = false;
        }

        None
    }
}

impl Display for Config {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{} | {}", show_state(Some(self.state)), self.tape)
    }
}

/**************************************/

#[derive(PartialEq, Eq, Hash, Clone)]
struct Span(Vec<Block>);

impl Span {
    fn blank(&self) -> bool {
        self.0.iter().all(|&block| block.color == 0)
    }

    fn len(&self) -> usize {
        self.0.iter().map(|block| block.count as Pos).sum()
    }

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn push(&mut self, print: Color, stepped: Count) {
        if self.is_empty() {
            self.0.insert(0, Block::new(print, stepped));
            return;
        }

        let block = &mut self.0[0];

        if block.color == print {
            block.add_count(stepped);
        } else {
            self.0.insert(0, Block::new(print, stepped));
        }
    }

    fn pull(
        &mut self,
        scan: Color,
        skip: bool,
    ) -> (Option<Color>, Count) {
        let stepped =
            (skip && !self.is_empty() && self.0[0].get_color() == scan)
                .then(|| self.0.remove(0))
                .as_ref()
                .map_or_else(|| 1, |block| 1 + block.get_count());

        let next_scan = if self.is_empty() {
            None
        } else {
            let next_pull = &mut self.0[0];

            let pull_color = next_pull.get_color();

            if next_pull.get_count() > 1 {
                next_pull.decrement();
            } else {
                self.0.remove(0);
            }

            Some(pull_color)
        };

        (next_scan, stepped)
    }

    fn take(&mut self) -> Color {
        assert!(!self.is_empty());

        let block = &mut self.0[0];

        let color = block.color;

        if block.count == 1 {
            self.0.remove(0);
        } else {
            block.decrement();
        }

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
    fn init(seg: Segments, pos: Pos) -> Self {
        assert!(seg >= 4);
        assert!(pos <= seg);

        let seg = seg as Count;
        let pos = pos as Count;
        let cells = (seg - 2) as Count;

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
            lspan: Span(lspan),
            rspan: Span(rspan),
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
}

impl Display for Tape {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(
            f,
            "{}",
            self.lspan
                .0
                .iter()
                .map(ToString::to_string)
                .rev()
                .chain(once(format!(
                    "[{}]",
                    self.scan.map_or_else(
                        || "-".to_owned(),
                        |scan| scan.to_string()
                    )
                )))
                .chain(self.rspan.0.iter().map(ToString::to_string))
                .collect::<Vec<_>>()
                .join(" ")
        )
    }
}

/**************************************/

type Halts = Set<State>;
type Spinouts = Dict<State, Shift>;

type Diffs = Vec<State>;
type Dirs = Dict<bool, Vec<State>>;

struct AnalyzedProg<'p> {
    prog: &'p CompProg,
    halts: Halts,
    spinouts: Spinouts,
    branches: Dict<State, (Diffs, Dirs)>,
}

impl<'p> AnalyzedProg<'p> {
    fn new(prog: &'p CompProg, (states, colors): Params) -> Self {
        let mut halts = Set::new();
        let mut spinouts = Dict::new();

        let mut branches = Dict::new();

        for state in 0..states {
            let mut diff = Set::new();
            let mut lefts = Set::new();
            let mut rights = Set::new();

            for color in 0..colors {
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

            branches.insert(
                state,
                (diff, Dict::from([(false, lefts), (true, rights)])),
            );
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
use crate::instrs::Parse as _;

#[cfg(test)]
impl Tape {
    #[track_caller]
    fn scan(&self) -> Option<Color> {
        if self.scan.is_none() {
            assert!(self.lspan.is_empty() || self.rspan.is_empty());
        }

        self.scan
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
    let prog = CompProg::from_str("1RB 1RC  0LA 0RA  0LB ...");

    let mut config = Config::init(4, 1);

    config.assert("A | [0] 0");

    let mut seen: Set<Config> = Set::new();

    while let Some(scan) = config.tape.scan() {
        seen.insert(config.clone());

        config.step(&prog[&(config.state, scan)]);
    }

    assert!(config.tape.side());

    config.assert("B | 0 1 [-]");

    assert_eq!(seen.len(), 6);
}

#[test]
fn test_reached_states() {
    let progs = [
        (("1RB 1RC  0LA 0RA  0LB ...", (3, 2)), vec![2], vec![]),
        (("1RB ...  1LB 0RC  1LC 1LA", (3, 2)), vec![0], vec![1, 2]),
        (("1RB ... ...  2LB 1RB 1LB", (2, 3)), vec![0], vec![1]),
        (("1RB 0RB ...  2LA ... 0LB", (2, 3)), vec![0, 1], vec![]),
        (
            ("1RB ... 0RB ...  2LB 3RA 0RA 0RA", (2, 4)),
            vec![0],
            vec![1],
        ),
    ];

    for ((prog, params), halts, spinouts) in progs {
        let comp = CompProg::from_str(prog);
        let prog = AnalyzedProg::new(&comp, params);

        assert_eq!(prog.halts, halts.into_iter().collect::<Set<_>>());

        assert_eq!(
            prog.spinouts.keys().copied().collect::<Set<_>>(),
            spinouts.into_iter().collect::<Set<_>>()
        );
    }
}

#[test]
fn test_cant_halt() {
    let results = [
        ("1RB ...  ... ...", (2, 2), None),
        ("1RB ...  1LB 0RC  1LC 1LA", (3, 2), None),
        ("1RB 1RC  0LA 0RA  0LB ...", (3, 2), Some(2)),
    ];

    for (prog, params, result) in results {
        assert_eq!(
            result,
            segment_cant_halt(&CompProg::from_str(prog), params, 2),
        );
    }
}

#[cfg(test)]
macro_rules! tape {
    (
        $ scan : expr,
        [ $ ( $ lspan : expr ), * ],
        [ $ ( $ rspan : expr ), * ]
    ) => {
        Tape {
            scan: Some( $ scan ),
            lspan: Span ( vec! [ $ ( Block::new( $ lspan.0, $ lspan.1) ), * ] ),
            rspan: Span ( vec! [ $ ( Block::new( $ rspan.0, $ rspan.1) ), * ] ),
        }
    };
}

#[cfg(test)]
impl Tape {
    #[track_caller]
    fn assert(&self, expected: &str) {
        assert_eq!(self.to_string(), expected);
    }
}

#[test]
fn test_step_edge() {
    let mut tape = tape! { 0, [], [(1, 1)] };

    tape.step(false, 1, false);

    tape.assert("[-] 1^2");
}

#[test]
fn test_skip() {
    let mut tape = tape! { 1, [(1, 20)], [] };
    tape.assert("1^20 [1]");
    tape.step(false, 0, true);
    tape.assert("[-] 0^21");

    let mut tape = tape! { 1, [(1, 20)], [] };
    tape.assert("1^20 [1]");
    tape.step(false, 1, true);
    tape.assert("[-] 1^21");

    let mut tape = tape! { 0, [(0, 1)], [] };
    tape.assert("0 [0]");
    tape.step(false, 1, true);
    tape.assert("[-] 1^2");
}
