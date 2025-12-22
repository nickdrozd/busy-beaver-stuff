use core::{fmt, iter::once};

use std::{
    collections::{BTreeMap as Dict, BTreeSet as Set},
    rc::Rc,
};

use crate::{
    Color, Instr, Prog, Shift, Slot, State, Steps,
    instrs::Parse as _,
    tape::{self, Alignment, Block as _, LilBlock as Block, Pos, Scan},
};

pub type Recs = usize;

const MAX_RECS: Recs = 2;
const MAX_STACK_DEPTH: usize = 28;

/**************************************/

#[derive(Debug)]
pub enum BackwardResult {
    Init,
    LinRec,
    Spinout,
    StepLimit,
    DepthLimit,
    Refuted(Steps),
}

use BackwardResult::*;

impl BackwardResult {
    pub const fn is_refuted(&self) -> bool {
        matches!(self, Refuted(_))
    }

    pub const fn is_settled(&self) -> bool {
        matches!(self, Refuted(_) | Init)
    }
}

/**************************************/

impl<const s: usize, const c: usize> Prog<s, c> {
    pub fn cant_halt(&self, steps: Steps) -> BackwardResult {
        let (entrypoints, idx) = self.build_entrypoints_and_indices();

        let slots = self.halt_slots_disp_side(&idx);

        cant_reach(self, steps, slots, Some(entrypoints), halt_configs)
    }

    pub fn cant_blank(&self, steps: Steps) -> BackwardResult {
        if self.cant_blank_by_color_graph() {
            return BackwardResult::Refuted(0);
        }

        cant_reach(self, steps, self.erase_slots(), None, erase_configs)
    }

    pub fn cant_spin_out(&self, steps: Steps) -> BackwardResult {
        cant_reach(self, steps, self.zr_shifts(), None, zr_configs)
    }
}

/**************************************/

type Configs = Vec<Config>;
type BlankStates = Set<State>;

type Entry = (Slot, (Color, Shift));
type Entries = Vec<Entry>;
type Entrypoints = Dict<State, (Entries, Entries)>;

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

    let mut configs = get_configs(&slots);

    let mut blanks = get_blanks(&configs);

    let mut seen: Dict<Key, Vec<Tape>> = Dict::new();

    for step in 1..=steps {
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

        configs = match step_configs(valid_steps, &mut blanks) {
            Err(err) => return err,
            Ok(stepped) => {
                let mut kept = Configs::new();
                for cfg in stepped {
                    let k = cfg_key(&cfg);
                    let entry = seen.entry(k).or_default();
                    if antichain_insert(entry, cfg.tape.clone()) {
                        kept.push(cfg);
                    }
                }
                kept
            },
        };
    }

    StepLimit
}

type ValidatedSteps = Vec<(Vec<Instr>, Config)>;

fn get_valid_steps(
    configs: &mut Configs,
    entrypoints: &Entrypoints,
) -> ValidatedSteps {
    let mut checked = ValidatedSteps::new();

    for config in configs.drain(..) {
        let Config { state, tape, .. } = &config;

        let mut steps = vec![];

        let Some((same, diff)) = entrypoints.get(state) else {
            assert!(*state == 0);
            continue;
        };

        for &((next_state, color), (print, shift)) in diff {
            if !tape.is_valid_step(shift, print) {
                continue;
            }

            steps.push((color, shift, next_state));
        }

        for &((next_state, color), (print, shift)) in same {
            if !tape.is_valid_step(shift, print) {
                continue;
            }

            if !tape.is_spinout(shift, color) {
                steps.push((color, shift, next_state));
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
    let mut checked_entries = Entries::new();

    let scan = config.tape.scan;

    for entries in [diff, same] {
        for &entry @ ((state, color), (_, shift)) in entries {
            if state == config.state && shift == push && scan == color {
                continue;
            }

            checked_entries.push(entry);
        }
    }

    if checked_entries.is_empty() {
        return None;
    }

    let mut tape = config.tape.clone();

    tape.push_indef(push);

    let mut steps = vec![];

    for ((state, color), (print, shift)) in checked_entries {
        if !tape.is_valid_step(shift, print) {
            continue;
        }

        steps.push((color, shift, state));
    }

    if steps.is_empty() {
        return None;
    }

    let next_config = Config::new(config.state, tape);

    #[cfg(debug_assertions)]
    println!("~ | {next_config}");

    Some((steps, next_config))
}

fn step_configs(
    configs: ValidatedSteps,
    blanks: &mut BlankStates,
) -> Result<Configs, BackwardResult> {
    let configs = branch_indef(configs)?;

    let mut stepped = Configs::new();

    for (instrs, config) in configs {
        let config = Rc::new(config);

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

            let next_config = Config::descendant(state, tape, &config)?;

            stepped.push(next_config);
        }
    }

    Ok(stepped)
}

fn branch_indef(
    configs: ValidatedSteps,
) -> Result<ValidatedSteps, BackwardResult> {
    let mut branched = ValidatedSteps::new();

    for (instrs, config) in configs {
        let mut indef_left = vec![];
        let mut indef_right = vec![];

        for instr @ &(_, shift, state) in &instrs {
            if config.tape.pulls_indef(shift) {
                if state == config.state {
                    return Err(Spinout);
                }

                if shift {
                    &mut indef_left
                } else {
                    &mut indef_right
                }
                .push(*instr);
            }
        }

        if !indef_left.is_empty() {
            let mut count_1 = config.clone();
            count_1.tape.lspan.set_head_to_one();

            branched.push((indef_left, count_1));
        }

        if !indef_right.is_empty() {
            let mut count_1 = config.clone();
            count_1.tape.rspan.set_head_to_one();

            branched.push((indef_right, count_1));
        }

        branched.push((instrs, config));
    }

    Ok(branched)
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

fn get_blanks(configs: &Configs) -> BlankStates {
    configs
        .iter()
        .filter_map(|cfg| cfg.tape.blank().then_some(cfg.state))
        .collect()
}

/**************************************/

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
    recs: Recs,
    prev: Option<Rc<Self>>,
}

impl Config {
    const fn new(state: State, tape: Tape) -> Self {
        Self {
            state,
            tape,
            recs: 0,
            prev: None,
        }
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

    fn descendant(
        state: State,
        tape: Tape,
        prev: &Rc<Self>,
    ) -> Result<Self, BackwardResult> {
        let mut config = Self {
            state,
            tape,
            recs: prev.recs,
            prev: Some(Rc::clone(prev)),
        };

        if config.lin_rec() {
            config.recs += 1;

            if config.recs > MAX_RECS {
                return Err(LinRec);
            }
        }

        Ok(config)
    }

    fn lin_rec(&self) -> bool {
        let head = self.tape.head();
        let mut leftmost = head;
        let mut rightmost = head;

        let mut current = self.prev.as_deref();

        while let Some(config) = current {
            let pos = config.tape.head();

            if pos < leftmost {
                leftmost = pos;
            } else if rightmost < pos {
                rightmost = pos;
            }

            if self.state == config.state
                && self.tape.aligns_with(
                    &config.tape,
                    leftmost,
                    rightmost,
                )
            {
                return true;
            }

            current = config.prev.as_deref();
        }

        false
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

type SpanT = tape::Span<Block>;

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

    const fn end_str(&self) -> &str {
        match self.end {
            TapeEnd::Blanks => "0+",
            TapeEnd::Unknown => "?",
        }
    }

    fn blank(&self) -> bool {
        self.span.iter().all(Block::blank)
    }

    const fn len(&self) -> usize {
        self.span.len()
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

        match block.count {
            1 => {
                self.span.pop_block();
            },
            0 => {},
            _ => {
                block.decrement();
            },
        }
    }

    fn push_single(&mut self, color: Color) {
        match self.span.first_mut() {
            Some(block) if block.color == color && block.count != 0 => {
                block.count += 1;
            },
            None if color == 0 && self.end == TapeEnd::Blanks => {},
            _ => {
                self.span.push_block(color, 1);
            },
        }
    }

    fn push_indef(&mut self, color: Color) {
        if color == 0
            && self.span.blank()
            && self.end == TapeEnd::Blanks
        {
            return;
        }

        self.span.push_block(color, 0);
    }

    fn set_head_to_one(&mut self) {
        self.span.first_mut().unwrap().count = 1;
    }
}

/**************************************/

#[derive(Clone, PartialEq, Eq, Hash)]
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

    fn blank(&self) -> bool {
        self.scan == 0 && self.lspan.blank() && self.rspan.blank()
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

    fn pulls_indef(&self, shift: Shift) -> bool {
        let pull = if shift { &self.lspan } else { &self.rspan };

        let Some(block) = pull.span.first() else {
            return false;
        };

        block.is_indef()
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
}

impl Alignment for Tape {
    fn head(&self) -> Pos {
        self.head
    }

    fn l_len(&self) -> usize {
        self.lspan.len()
    }

    fn r_len(&self) -> usize {
        self.rspan.len()
    }

    fn l_eq(&self, prev: &Self) -> bool {
        self.lspan == prev.lspan
    }

    fn r_eq(&self, prev: &Self) -> bool {
        self.rspan == prev.rspan
    }

    fn l_compare_take(&self, prev: &Self, take: usize) -> bool {
        self.lspan.span.compare_take(&prev.lspan.span, take)
    }

    fn r_compare_take(&self, prev: &Self, take: usize) -> bool {
        self.rspan.span.compare_take(&prev.rspan.span, take)
    }
}

/**************************************/

#[cfg(test)]
impl From<&str> for Block {
    fn from(s: &str) -> Self {
        let (color, count) = if s.ends_with("..") {
            (s.trim_end_matches("..").parse().unwrap(), 0)
        } else if s.contains('^') {
            let parts: Vec<&str> = s.split('^').collect();
            (parts[0].parse().unwrap(), parts[1].parse().unwrap())
        } else {
            (s.parse().unwrap(), 1)
        };

        Self { color, count }
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
            span.span.push_block(block.color, block.count);
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

    tape.assert("0+ 1 0.. 1.. 0.. 0 [0] ?");
}

/**************************************/

use core::array;
use std::collections::VecDeque;

type Adj<const S: usize> = [Vec<usize>; S];
type Preds<const S: usize> = [[Vec<usize>; 2]; S]; // preds[v][dir] -> u
type Writers<const C: usize> = [[Vec<usize>; 2]; C]; // writers[color][dir] -> v
type NextDir<const S: usize> = [[Vec<usize>; 2]; S]; // next[u][dir] -> v
type Indices<const S: usize, const C: usize> =
    (Adj<S>, Preds<S>, Writers<C>, NextDir<S>);

fn indices_new<const S: usize, const C: usize>() -> Indices<S, C> {
    (
        array::from_fn(|_| Vec::new()),
        array::from_fn(|_| array::from_fn(|_| Vec::new())),
        array::from_fn(|_| array::from_fn(|_| Vec::new())),
        array::from_fn(|_| array::from_fn(|_| Vec::new())),
    )
}

fn indices_add<const S: usize, const C: usize>(
    (adj, preds, writers, next): &mut Indices<S, C>,
    u: usize,
    v: usize,
    dir: usize, // 0=L, 1=R   (Shift false/true)
    pr: usize,  // printed color
) {
    adj[u].push(v);
    preds[v][dir].push(u);
    writers[pr][dir].push(v);
    next[u][dir].push(v);
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

        // find root
        let mut root = None;
        for v in 0..S {
            if ((mask >> v) & 1) == 1 {
                root = Some(v);
                break;
            }
        }
        let Some(root) = root else { continue };

        let mut in_comp = [false; S];
        for v in 0..S {
            in_comp[v] = ((mask >> v) & 1) == 1;
        }

        let mut dist: [Option<i32>; S] = [(); S].map(|()| None);
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
/// If `negate` is true, weights are negated => detects positive cycles of original graph.
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
    fn build_entrypoints_and_indices(
        &self,
    ) -> (Entrypoints, Indices<S, C>) {
        let mut entrypoints = Entrypoints::new();
        let mut idx = indices_new::<S, C>();

        for (slot @ (read, _scan), &(pr, sh, next_state)) in self.iter()
        {
            let u = read as usize;
            let v = next_state as usize;
            if u >= S || v >= S {
                continue;
            }

            // entrypoints (your existing structure)
            let (same, diff) =
                entrypoints.entry(next_state).or_default();
            (if read == next_state { same } else { diff })
                .push((slot, (pr, sh)));

            // indices
            let dir = usize::from(sh); // true=R(1), false=L(0)
            let pr = pr as usize;
            if pr < C {
                indices_add::<S, C>(&mut idx, u, v, dir, pr);
            }
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

    let mut lo = 0;
    let mut any = false;
    for v in 0..S {
        if ((mask >> v) & 1) == 0 {
            continue;
        }
        if d[v] >= 900_000 {
            continue;
        }
        any = true;
        if d[v] < lo {
            lo = d[v];
        }
    }
    if !any {
        return 0;
    }

    // We need to search displacements in [lo .. 0].
    // For S<=16 and no negative cycles, lo is typically >= -(S-1) (<= -15).
    // Keep a safe cap; if it somehow exceeds the cap, return
    // conservative "all nodes".
    #[expect(clippy::items_after_statements)]
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

    let mut q = std::collections::VecDeque::new();
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

impl TapeEnd {
    const fn subsumes(&self, other: &Self) -> bool {
        #[expect(clippy::match_same_arms)]
        match (self, other) {
            (Self::Unknown, _) => true,
            (Self::Blanks, Self::Blanks) => true,
            (Self::Blanks, Self::Unknown) => false,
        }
    }
}

#[expect(clippy::multiple_inherent_impl)]
impl Span {
    /// `self` subsumes `other` (self is more general / less constrained).
    fn subsumes(&self, other: &Self) -> bool {
        if !self.end.subsumes(&other.end) {
            return false;
        }
        self.span_subsumes(&other.span)
    }

    /// Compare two block-spans from the head outward (your Span stores blocks starting at head side).
    /// Rule (sound and simple):
    /// - colors must match positionally
    /// - count 0 means "indefinitely many" (≥1), so it subsumes any positive count
    /// - positive count must match exactly (conservative but sound)
    /// - if self runs out of blocks, it still subsumes if its end is Unknown or Blanks compatible
    fn span_subsumes(&self, other: &SpanT) -> bool {
        let mut it_a = self.span.iter(); // Blocks
        let mut it_b = other.iter();

        loop {
            match (it_a.next(), it_b.next()) {
                (None, None) => return true,
                (Some(_), None) => {
                    // self has extra constraints beyond other => does NOT subsume
                    return false;
                },
                (None, Some(_)) => {
                    // self ended earlier; it represents "nothing more specified".
                    // Allowed only if self end is Unknown or (Blanks and remaining other blocks are all blank)
                    match self.end {
                        TapeEnd::Unknown => return true,
                        TapeEnd::Blanks => {
                            // remaining blocks on other must all be blank
                            // we already consumed one from it_b via Some(_); check it + rest
                            // easiest: false here, conservative; or implement full scan.
                            return false;
                        },
                    }
                },
                (Some(a), Some(b)) => {
                    if a.color != b.color {
                        return false;
                    }
                    #[expect(clippy::match_same_arms)]
                    match (a.count, b.count) {
                        (0, 0) => {},           // both indefinite
                        (0, _) => {}, // self indefinite subsumes any b>=1
                        (_, 0) => return false, // self fixed cannot subsume indefinite
                        (ac, bc) if ac == bc => {},
                        _ => return false,
                    }
                },
            }
        }
    }
}

#[expect(clippy::multiple_inherent_impl)]
impl Tape {
    /// `self` subsumes `other` (self is more general / less constrained).
    fn subsumes(&self, other: &Self) -> bool {
        self.scan == other.scan
            && self.head == other.head
            && self.lspan.subsumes(&other.lspan)
            && self.rspan.subsumes(&other.rspan)
    }
}

type Key = (State, Color, Pos);

const fn cfg_key(cfg: &Config) -> Key {
    (cfg.state, cfg.tape.scan, cfg.tape.head)
}

/// Maintain an antichain of tapes under `subsumes`.
/// Returns true if `tape` was kept (i.e. not subsumed by existing).
fn antichain_insert(set: &mut Vec<Tape>, tape: Tape) -> bool {
    // If any existing subsumes new, drop new
    for old in set.iter() {
        if old.subsumes(&tape) {
            return false;
        }
    }

    // Remove any existing tapes subsumed by new
    set.retain(|old| !tape.subsumes(old));

    set.push(tape);
    true
}
