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

    let (halts, edges) = halts_and_edges(prog, params);

    (2..=segs).find(|seg| {
        #[cfg(all(not(test), debug_assertions))]
        println!();

        !all_segments_reached(prog, 2 + seg, &halts, &edges)
    })
}

/**************************************/

type Halts = Set<State>;
type Edges = Dict<State, Vec<(Shift, State)>>;

fn all_segments_reached(
    prog: &CompProg,
    seg: Segments,
    halts: &Halts,
    edges: &Edges,
) -> bool {
    let mut configs = Configs::new(seg, halts);

    'next_config: while let Some(mut config) = configs.next() {
        #[cfg(all(not(test), debug_assertions))]
        println!("{config}");

        while let Some(slot) = config.slot() {
            let Some(instr) = prog.get(&slot) else {
                if configs.check_reached(&config) {
                    return true;
                }

                continue 'next_config;
            };

            config.step(instr);

            if configs.check_seen(&config) {
                continue 'next_config;
            }
        }

        if configs.check_reached(&config) {
            return true;
        }

        configs.branch(config, edges);
    }

    false
}

fn halts_and_edges(
    prog: &CompProg,
    (states, colors): Params,
) -> (Halts, Edges) {
    let mut halts = Halts::new();
    let mut edges = Edges::new();

    for state in 0..states {
        let mut instrs = Set::new();

        for color in 0..colors {
            if let Some(&(_, shift, next)) = prog.get(&(state, color)) {
                instrs.insert((shift, next));
            } else {
                halts.insert(state);
            }
        }

        let mut instrs: Vec<_> = instrs.into_iter().collect();

        instrs.sort_unstable();

        edges.insert(state, instrs);
    }

    (halts, edges)
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
    fn new(seg: Segments, halts: &Halts) -> Self {
        Self {
            seg,
            todo: (0..seg).map(|pos| Config::init(seg, pos)).collect(),
            seen: Dict::new(),
            blanks: Dict::new(),
            reached: halts
                .iter()
                .map(|&state| (state, Set::new()))
                .collect(),
        }
    }

    fn check_seen(&mut self, Config { state, tape }: &Config) -> bool {
        if tape.blank() {
            let blanks = self.blanks.entry(*state).or_default();

            let pos = tape.pos();

            if blanks.contains(&pos) {
                return true;
            }

            blanks.insert(pos);
        } else {
            let seen = self.seen.entry(*state).or_default();

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
        Config { state, tape }: Config,
        edges: &Edges,
    ) {
        let side = tape.side();

        for &(shift, next_state) in &edges[&state] {
            if next_state != state {
                let next_tape = tape.clone();

                let config = Config::new(next_state, next_tape);

                self.todo.push(config);
            }

            if shift != side {
                let mut next_tape = tape.clone();

                next_tape.step_in(shift);

                let config = Config::new(next_state, next_tape);

                self.todo.push(config);
            }
        }
    }
}

impl Iterator for Configs {
    type Item = Config;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(config) = self.todo.pop() {
            if !self.check_seen(&config) {
                return Some(config);
            }
        }

        None
    }
}

/**************************************/

#[derive(PartialEq, Eq, Hash, Clone)]
struct Config {
    state: State,
    tape: Tape,
}

impl Config {
    #[expect(clippy::missing_const_for_fn)]
    fn new(state: State, tape: Tape) -> Self {
        Self { state, tape }
    }

    fn init(seg: Segments, pos: Pos) -> Self {
        Self::new(0, Tape::init(seg, pos))
    }

    fn slot(&self) -> Option<Slot> {
        Some((self.state, self.tape.scan?))
    }

    fn step(&mut self, &(print, shift, state): &Instr) {
        self.state = state;
        self.tape.step(shift, print);
    }
}

impl Display for Config {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{} | {}", show_state(Some(self.state)), self.tape)
    }
}

/**************************************/

#[derive(PartialEq, Eq, Hash, Clone)]
struct Tape {
    scan: Option<Color>,
    lspan: Vec<Color>,
    rspan: Vec<Color>,
}

impl Tape {
    fn init(seg: Segments, pos: Pos) -> Self {
        assert!(seg >= 4);
        assert!(pos <= seg);

        let cells = seg - 2;

        if pos == 0 {
            Self {
                scan: None,
                lspan: vec![],
                rspan: vec![0; cells],
            }
        } else if pos == seg - 1 {
            Self {
                scan: None,
                lspan: vec![0; cells],
                rspan: vec![],
            }
        } else {
            Self {
                scan: Some(0),
                lspan: vec![0; pos - 1],
                rspan: vec![0; cells - pos],
            }
        }
    }

    fn blank(&self) -> bool {
        matches!(self.scan, Some(0) | None)
            && self
                .lspan
                .iter()
                .chain(self.rspan.iter())
                .all(|&c| c == 0)
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

        self.scan = if pull.is_empty() {
            None
        } else {
            Some(pull.remove(0))
        };
    }

    fn step(&mut self, shift: Shift, print: Color) {
        let (pull, push) = if shift {
            (&mut self.rspan, &mut self.lspan)
        } else {
            (&mut self.lspan, &mut self.rspan)
        };

        push.insert(0, print);

        self.scan = if pull.is_empty() {
            None
        } else {
            Some(pull.remove(0))
        };
    }
}

impl Display for Tape {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(
            f,
            "{}",
            self.lspan
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
                .chain(self.rspan.iter().map(ToString::to_string))
                .collect::<Vec<_>>()
                .join(" ")
        )
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
            halts_and_edges(&CompProg::from_str(prog), params).0,
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
