use core::{
    fmt::{Display, Formatter, Result},
    iter::once,
};

use std::collections::{BTreeMap as Dict, HashSet as Set};

use crate::instrs::{
    show_state, Color, CompProg, Instr, Params, Shift, Slot, State,
};

type Segments = usize;

const MAX_DEPTH: usize = 4_000;

/**************************************/

enum SearchResult {
    Limit,
    Halted,
    Repeat,
    Spinout,
    Reached,
    Nothing,
}

#[expect(clippy::enum_glob_use)]
use SearchResult::*;

pub fn segment_cant_halt(
    prog: &CompProg,
    params: Params,
    segs: Segments,
) -> Option<Segments> {
    assert!(segs >= 2);

    let prog = AnalyzedProg::new(prog, params);

    for seg in 2..=segs {
        match all_segments_reached(&prog, 2 + seg) {
            Reached => continue,
            Limit | Halted => return None,
            Nothing | Spinout | Repeat => return Some(seg),
        }
    }

    None
}

/**************************************/

fn all_segments_reached(
    prog: &AnalyzedProg,
    seg: Segments,
) -> SearchResult {
    let mut configs = Configs::new(seg, prog);

    #[cfg(all(not(test), debug_assertions))]
    println!();

    while let Some(mut config) = configs.next() {
        #[cfg(all(not(test), debug_assertions))]
        println!("{config}");

        if let Some(result) = config.run_to_edge(prog, &mut configs) {
            if matches!(result, Halted | Repeat | Reached | Spinout) {
                return result;
            }

            continue;
        }

        if configs.check_reached(&config) {
            return Reached;
        }

        let (diffs, dirs) = &prog.branches[&config.state];

        configs.branch_in(&config.tape, dirs);

        configs.branch_out(config, diffs);

        if configs.check_depth() {
            return Limit;
        }
    }

    Nothing
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
    fn new(seg: Segments, prog: &AnalyzedProg) -> Self {
        Self {
            seg,
            todo: vec![],
            seen: Dict::new(),
            blanks: Dict::new(),
            reached: prog
                .halts
                .iter()
                .map(|&state| (state, Set::new()))
                .collect(),
        }
    }

    fn add_todo(&mut self, config: Config) {
        self.todo.push(config);
    }

    fn check_depth(&self) -> bool {
        self.todo.len() > MAX_DEPTH
            || self.seen.values().any(|seen| seen.len() > MAX_DEPTH)
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
    ) -> Option<bool> {
        let blank = tape.blank();

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

    fn check_reached(&mut self, config: &Config) -> bool {
        let Some(reached) = self.reached.get_mut(&config.state) else {
            return false;
        };

        reached.insert(config.tape.pos());

        reached.len() == self.seg
    }

    fn branch_in(&mut self, tape: &Tape, dirs: &Dirs) {
        let shift = !tape.side();

        for &state in &dirs[&shift] {
            let mut next_tape = tape.clone();

            next_tape.step_in(shift);

            let Some(init) = self.check_seen(state, &next_tape) else {
                continue;
            };

            let config = Config::new(state, next_tape, init);

            self.add_todo(config);
        }
    }

    fn branch_out(&mut self, mut config: Config, diffs: &Diffs) {
        let tape = &config.tape;

        let Some((last_next, diffs)) = diffs.split_last() else {
            return;
        };

        for &state in diffs {
            let Some(init) = self.check_seen(state, tape) else {
                continue;
            };

            let next_tape = tape.clone();

            let config = Config::new(state, next_tape, init);

            self.add_todo(config);
        }

        if let Some(init) = self.check_seen(*last_next, tape) {
            config.state = *last_next;

            config.init = init;

            self.add_todo(config);
        }
    }
}

impl Iterator for Configs {
    type Item = Config;

    fn next(&mut self) -> Option<Self::Item> {
        self.todo.pop().or_else(|| self.next_init())
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
        self.state = state;
        self.tape.step(shift, print);
    }

    fn spinout(&self, &(_, shift, state): &Instr) -> bool {
        self.state == state && self.tape.at_edge(shift)
    }

    #[expect(clippy::unwrap_in_result)]
    fn run_to_edge(
        &mut self,
        prog: &AnalyzedProg,
        configs: &mut Configs,
    ) -> Option<SearchResult> {
        self.tape.scan?;

        let mut step = false;
        let mut copy = self.clone();

        while let Some(slot) = self.slot() {
            let Some(instr) = prog.get(&slot) else {
                if self.init {
                    return Some(Halted);
                }

                if configs.check_reached(self) {
                    return Some(Reached);
                }

                return Some(Nothing);
            };

            if self.init && self.spinout(instr) {
                return Some(Spinout);
            }

            self.step(instr);

            if !step {
                step = true;
                continue;
            }

            let instr = prog.get(&copy.slot().unwrap()).unwrap();

            copy.step(instr);

            if copy == *self {
                return Some(if self.init { Repeat } else { Nothing });
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
struct Span(Vec<Color>);

impl Span {
    fn blank(&self) -> bool {
        self.0.iter().all(|&c| c == 0)
    }

    fn len(&self) -> usize {
        self.0.len()
    }

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn push(&mut self, print: Color) {
        self.0.insert(0, print);
    }

    fn pull(&mut self) -> Option<Color> {
        if self.is_empty() {
            None
        } else {
            Some(self.0.remove(0))
        }
    }

    fn take(&mut self) -> Color {
        assert!(!self.is_empty());

        self.0.remove(0)
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

        let cells = seg - 2;

        let (scan, lspan, rspan) = if pos == 0 {
            (None, vec![], vec![0; cells])
        } else if pos == seg - 1 {
            (None, vec![0; cells], vec![])
        } else {
            (Some(0), vec![0; pos - 1], vec![0; cells - pos])
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

    fn step(&mut self, shift: Shift, print: Color) {
        let (pull, push) = if shift {
            (&mut self.rspan, &mut self.lspan)
        } else {
            (&mut self.lspan, &mut self.rspan)
        };

        push.push(print);

        self.scan = pull.pull();
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

type Diffs = Vec<State>;
type Dirs = Dict<bool, Vec<State>>;

struct AnalyzedProg<'p> {
    prog: &'p CompProg,
    halts: Set<State>,
    branches: Dict<State, (Diffs, Dirs)>,
}

impl<'p> AnalyzedProg<'p> {
    fn get(&self, slot: &Slot) -> Option<&Instr> {
        self.prog.get(slot)
    }

    fn new(prog: &'p CompProg, (states, colors): Params) -> Self {
        let mut halts = Set::new();

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

                if next != state {
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
        "A | [-] 0 0 0 0 0",
        "A | [0] 0 0 0 0",
        "A | 0 [0] 0 0 0",
        "A | 0 0 [0] 0 0",
        "A | 0 0 0 [0] 0",
        "A | 0 0 0 0 [0]",
        "A | 0 0 0 0 0 [-]",
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

    config.assert("A | 0 0 0 0 0 [-]");

    config.tape.step_in(false);

    config.assert("A | 0 0 0 0 [0]");
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
fn test_halt_states() {
    let progs = [
        (("1RB 1RC  0LA 0RA  0LB ...", (3, 2)), vec![2]),
        (("1RB ...  1LB 0RC  1LC 1LA", (3, 2)), vec![0]),
        (("1RB ... ...  2LB 1RB 1LB", (2, 3)), vec![0]),
        (("1RB 0RB ...  2LA ... 0LB", (2, 3)), vec![0, 1]),
        (("1RB ... 0RB ...  2LB 3RA 0RA 0RA", (2, 4)), vec![0]),
    ];

    for ((prog, params), states) in progs {
        assert_eq!(
            AnalyzedProg::new(&CompProg::from_str(prog), params).halts,
            states.into_iter().collect::<Set<_>>(),
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

#[test]
fn test_step_edge() {
    let mut tape = Tape {
        scan: Some(0),
        lspan: Span(vec![]),
        rspan: Span(vec![1]),
    };

    tape.step(false, 1);

    assert_eq!(tape.to_string(), "[-] 1 1");
}
