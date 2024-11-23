use core::{
    fmt::{Display, Formatter, Result},
    iter::once,
};

use std::collections::{BTreeMap as Dict, HashSet as Set};

use crate::instrs::{
    show_state, Color, CompProg, Instr, Params, Shift, Slot, State,
};

type Segments = usize;

/**************************************/

pub fn segment_cant_halt(
    prog: &CompProg,
    params: Params,
    segs: Segments,
) -> Option<Segments> {
    assert!(segs >= 2);

    let prog = AnalyzedProg::new(prog, params);

    for seg in 2..=segs {
        match all_segments_reached(&prog, 2 + seg) {
            Some(SearchResult::Reached) => continue,
            Some(SearchResult::Halted) => return None,
            None | Some(SearchResult::Spinout) => return Some(seg),
        }
    }

    None
}

/**************************************/

enum SearchResult {
    Halted,
    Spinout,
    Reached,
}

fn all_segments_reached(
    prog: &AnalyzedProg,
    seg: Segments,
) -> Option<SearchResult> {
    let mut configs = Configs::new(seg, prog);

    #[cfg(all(not(test), debug_assertions))]
    println!();

    'next_config: while let Some(mut config) = configs.next() {
        #[cfg(all(not(test), debug_assertions))]
        println!("{config}");

        while let Some(slot) = config.slot() {
            let Some(instr) = prog.get(&slot) else {
                if config.init {
                    return Some(SearchResult::Halted);
                }

                if configs.check_reached(&config) {
                    return Some(SearchResult::Reached);
                }

                continue 'next_config;
            };

            if config.init && config.spinout(instr) {
                return Some(SearchResult::Spinout);
            }

            config.step(instr);

            if configs.check_seen(config.state, &config.tape) {
                continue 'next_config;
            }
        }

        if configs.check_reached(&config) {
            return Some(SearchResult::Reached);
        }

        configs.branch(config, prog);
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

    fn next_init(&mut self) -> Option<Config> {
        let blanks = self.blanks.entry(0).or_default();

        let pos = (0..self.seg).find(|pos| !blanks.contains(pos))?;

        blanks.insert(pos);

        Some(Config::init(self.seg, pos))
    }

    fn check_seen(&mut self, state: State, tape: &Tape) -> bool {
        if tape.blank() {
            let blanks = self.blanks.entry(state).or_default();

            let pos = tape.pos();

            if blanks.contains(&pos) {
                return true;
            }

            blanks.insert(pos);
        } else {
            let seen = self.seen.entry(state).or_default();

            if seen.contains(tape) {
                return true;
            }

            seen.insert(tape.clone());
        }

        false
    }

    fn check_reached(&mut self, config: &Config) -> bool {
        let Some(reached) = self.reached.get_mut(&config.state) else {
            return false;
        };

        reached.insert(config.tape.pos());

        reached.len() == self.seg
    }

    fn branch(
        &mut self,
        Config { state, tape, .. }: Config,
        prog: &AnalyzedProg,
    ) {
        let side = tape.side();

        for &(shift, next_state) in &prog.edges[&state] {
            if next_state != state
                && !self.check_seen(next_state, &tape)
            {
                let next_tape = tape.clone();

                let config = Config::new(next_state, next_tape, false);

                self.todo.push(config);
            }

            if shift != side {
                let mut next_tape = tape.clone();

                next_tape.step_in(shift);

                if !self.check_seen(next_state, &next_tape) {
                    let init = next_state == 0 && next_tape.blank();

                    let config =
                        Config::new(next_state, next_tape, init);

                    self.todo.push(config);
                }
            }
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

struct AnalyzedProg<'p> {
    prog: &'p CompProg,
    halts: Set<State>,
    edges: Dict<State, Vec<(Shift, State)>>,
}

impl<'p> AnalyzedProg<'p> {
    fn get(&self, slot: &Slot) -> Option<&Instr> {
        self.prog.get(slot)
    }

    fn new(prog: &'p CompProg, (states, colors): Params) -> Self {
        let mut halts = Set::new();
        let mut edges = Dict::new();

        for state in 0..states {
            let mut instrs = Set::new();

            for color in 0..colors {
                if let Some(&(_, shift, next)) =
                    prog.get(&(state, color))
                {
                    instrs.insert((shift, next));
                } else {
                    halts.insert(state);
                }
            }

            let mut instrs: Vec<_> = instrs.into_iter().collect();

            instrs.sort_unstable();

            edges.insert(state, instrs);
        }

        Self { prog, halts, edges }
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
