use core::{
    cell::RefCell,
    fmt::{self, Display},
};

use ahash::{AHashMap as Dict, AHashSet as Set};

use crate::{Color, Instr, Shift, Slot, State, instrs::Parse};

/**************************************/

pub struct Prog<const states: usize, const colors: usize> {
    table: [[Option<Instr>; colors]; states],
}

impl<const s: usize, const c: usize> From<&str> for Prog<s, c> {
    fn from(prog: &str) -> Self {
        let mut out = Self::new();

        for (i, colors) in prog.trim().split("  ").enumerate().take(s) {
            for (j, instr) in colors.split(' ').enumerate().take(c) {
                if let Some(parsed) = Parse::read(instr) {
                    #[expect(clippy::cast_possible_truncation)]
                    let slot = (i as State, j as Color);
                    out.insert(&slot, &parsed);
                }
            }
        }

        out
    }
}

impl<const s: usize, const c: usize> Display for Prog<s, c> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (states, colors) = self.max_reached();

        for state in 0..=states {
            if 0 < state {
                write!(f, "  ")?;
            }

            for color in 0..=colors {
                if 0 < color {
                    write!(f, " ")?;
                }

                write!(f, "{}", self.get(&(state, color)).show())?;
            }
        }

        Ok(())
    }
}

impl<const states: usize, const colors: usize> Prog<states, colors> {
    const fn new() -> Self {
        Self {
            table: [[None; colors]; states],
        }
    }

    pub const fn init_norm() -> Self {
        let mut prog = Self::new();

        prog.insert(&(0, 0), &(1, true, 1));

        prog
    }

    pub fn print(&self) {
        println!("{self}");
    }

    pub fn get(&self, &(state, color): &Slot) -> Option<&Instr> {
        unsafe {
            self.table
                .get_unchecked(state as usize)
                .get_unchecked(color as usize)
        }
        .as_ref()
    }

    pub const fn insert(
        &mut self,
        &(state, color): &Slot,
        instr: &Instr,
    ) {
        self.table[state as usize][color as usize] = Some(*instr);
    }

    pub const fn remove(&mut self, &(state, color): &Slot) {
        self.table[state as usize][color as usize] = None;
    }

    #[expect(clippy::cast_possible_truncation)]
    pub fn iter(&self) -> impl Iterator<Item = (Slot, &Instr)> {
        self.table.iter().enumerate().flat_map(|(state, cos)| {
            cos.iter().enumerate().filter_map(move |(color, maybe)| {
                maybe
                    .as_ref()
                    .map(|instr| ((state as u8, color as u8), instr))
            })
        })
    }

    pub fn instrs(&self) -> impl Iterator<Item = &Instr> {
        self.table
            .iter()
            .flat_map(|cos| cos.iter().filter_map(Option::as_ref))
    }

    pub fn max_reached(&self) -> (State, Color) {
        let mut st = 1;
        let mut co = 1;

        for &(pr, _, tr) in self.instrs() {
            if co < pr {
                co = pr;
            }

            if st < tr {
                st = tr;
            }
        }

        (st, co)
    }

    pub fn reached_params(&self) -> (usize, usize) {
        let (state, color) = self.max_reached();

        (1 + state as usize, 1 + color as usize)
    }

    pub fn is_complete(&self) -> bool {
        let (s, c) = self.reached_params();

        s == states && c == colors
    }

    pub fn halt_slots(&self) -> Set<Slot> {
        let (states_reached, colors_reached) = self.max_reached();

        (0..=states_reached)
            .flat_map(|state| {
                (0..=colors_reached).map(move |color| (state, color))
            })
            .filter(|slot @ &(state, color)| {
                self.get(slot).is_none()
                    && (color == 0
                        || self.reaches_both_sides(state, color))
            })
            .collect()
    }

    pub fn erase_slots(&self) -> Set<Slot> {
        self.iter()
            .filter_map(|(slot @ (_, co), &(pr, _, _))| {
                (co != 0 && pr == 0).then_some(slot)
            })
            .collect()
    }

    pub fn zr_shifts(&self) -> Set<(State, Shift)> {
        self.iter()
            .filter_map(|(slot, &(_, sh, st))| {
                (slot == (st, 0)).then_some((st, sh))
            })
            .collect()
    }

    pub fn twostep_slots(&self) -> Set<(Slot, Slot)> {
        self.iter()
            .filter_map(|(slot @ (st, co), &(pr, sh, tr))| {
                (sh && pr == co).then_some((slot, st, tr))
            })
            .flat_map(|(left, l_st, r_st)| {
                self.iter().filter_map(
                    move |(right @ (st, co), &(pr, sh, tr))| {
                        (!sh && st == r_st && pr == co && tr == l_st)
                            .then_some((left, right))
                    },
                )
            })
            .collect()
    }

    #[expect(clippy::cast_possible_truncation)]
    pub fn blank_loops(&self) -> Set<(State, Shift)> {
        let mut next_r: [Option<State>; states] = [None; states];
        let mut next_l: [Option<State>; states] = [None; states];

        for ((st, co), &(_, sh, tr)) in self.iter() {
            if co != 0 {
                continue;
            }

            if sh {
                next_r[st as usize] = Some(tr);
            } else {
                next_l[st as usize] = Some(tr);
            }
        }

        fn collect<const states: usize>(
            next: &[Option<State>; states],
            sh: Shift,
            out: &mut Set<(State, Shift)>,
        ) {
            let mut done = [false; states];
            let mut seen_run = [0; states];
            let mut seen_step = [0; states];
            let mut run_id: u32 = 1;
            let mut path: Vec<State> = vec![];

            for start in 0..states {
                if done[start] {
                    continue;
                }

                path.clear();
                let mut cur = start as State;

                loop {
                    let cu = cur as usize;

                    if done[cu] {
                        break;
                    }

                    if seen_run[cu] == run_id {
                        let begin = seen_step[cu];
                        let cyc = &path[begin..];
                        if 2 <= cyc.len()
                            && let Some(&min_st) = cyc.iter().min()
                        {
                            out.insert((min_st, sh));
                        }
                        break;
                    }

                    seen_run[cu] = run_id;
                    seen_step[cu] = path.len();
                    path.push(cur);

                    let Some(nx) = next[cu] else {
                        break;
                    };

                    cur = nx;
                }

                for &st in &path {
                    done[st as usize] = true;
                }

                run_id += 1;
            }
        }

        let mut out: Set<(State, Shift)> = Set::new();

        collect(&next_r, true, &mut out);
        collect(&next_l, false, &mut out);

        out
    }

    pub fn reaches_both_sides(&self, st: State, co: Color) -> bool {
        let mut side = None;

        for &(pr, sh, tr) in self.instrs() {
            if pr != co && tr != st {
                continue;
            }

            let Some(sd) = side else {
                side = Some(sh);
                continue;
            };

            if sd != sh {
                return true;
            }
        }

        false
    }
}

/**************************************/

#[cfg(test)]
macro_rules! assert_show {
    ($prog:literal, ($s:literal, $c:literal), $show:literal) => {
        assert_eq!(Prog::<$s, $c>::from($prog).to_string(), $show);
    };
}

#[test]
fn test_sparse_show() {
    assert_show!(
        "1RB 2LB 2LB ...  1LA 2RB 2LA ...",
        (2, 4),
        "1RB 2LB 2LB  1LA 2RB 2LA"
    );

    assert_show!(
        "1RB 0LB  0LC 0RA  1LA 1LC  ... ...",
        (4, 2),
        "1RB 0LB  0LC 0RA  1LA 1LC"
    );

    assert_show!(
        "1RB 1LD ... ... ... ... ... ...  0RC 0LA ... ... ... ... ... ...  0LC 1LA ... ... ... ... ... ...  0LA 0LA ... ... ... ... ... ...  ... ... ... ... ... ... ... ...  ... ... ... ... ... ... ... ...  ... ... ... ... ... ... ... ...  ... ... ... ... ... ... ... ...",
        (8, 8),
        "1RB 1LD  0RC 0LA  0LC 1LA  0LA 0LA"
    );

    assert_show!(
        "1RB ... ... ... ... ... ... ...  2LB 0LC ... ... ... ... ... ...  0LD ... ... ... ... ... ... ...  0LE ... ... ... ... ... ... ...  0LF ... ... ... ... ... ... ...  1LG ... ... ... ... ... ... ...  3RC ... ... ... ... ... ... ...  ... ... ... ... ... ... ... ...",
        (8, 8),
        "1RB ... ... ...  2LB 0LC ... ...  0LD ... ... ...  0LE ... ... ...  0LF ... ... ...  1LG ... ... ...  3RC ... ... ..."
    );
}

#[test]
fn test_halt_slots() {
    assert_eq!(
        Prog::<3, 2>::from("1RB ...  0RC ...  0LA ...").halt_slots(),
        Set::from([(0, 1)]),
    );

    assert_eq!(
        Prog::<2, 3>::from("1RB 0LA ...  2LA ... ...").halt_slots(),
        Set::from([(1, 2)]),
    );

    assert_eq!(
        Prog::<8, 8>::from("1RB 1LD ... ... ... ... ... ...  0RC 0LA ... ... ... ... ... ...  0LC 1LA ... ... ... ... ... ...  0LA 0LA ... ... ... ... ... ...  ... ... ... ... ... ... ... ...  ... ... ... ... ... ... ... ...  ... ... ... ... ... ... ... ...  ... ... ... ... ... ... ... ...").halt_slots(),
        Set::from([]),
    );

    assert_eq!(
        Prog::<8, 8>::from("1RB ... ... ... ... ... ... ...  2LB 0LC ... ... ... ... ... ...  0LD ... ... ... ... ... ... ...  0LE ... ... ... ... ... ... ...  0LF ... ... ... ... ... ... ...  1LG ... ... ... ... ... ... ...  3RC ... ... ... ... ... ... ...  ... ... ... ... ... ... ... ...").halt_slots(),
        Set::from([(0, 1), (1, 2), (1, 3), (2, 1), (2, 2), (2, 3), (3, 1), (3, 3), (4, 1), (4, 3), (5, 1), (5, 3), (6, 1), (6, 3)]),
    );
}

#[test]
fn test_twostep_slots() {
    assert_eq!(
        Prog::<4, 2>::from("1RB 0LA  0RC 1LC  1LA 1RD  0LC 0RD")
            .twostep_slots(),
        Set::from([((2, 1), (3, 0))]),
    );

    assert_eq!(
        Prog::<4, 2>::from("1RB 1LD  0RC 1RC  1LA 1LB  0RA 0LD")
            .twostep_slots(),
        Set::from([
            ((1, 0), (2, 1)),
            ((1, 1), (2, 1)),
            ((3, 0), (0, 1)),
        ]),
    );

    assert_eq!(
        Prog::<2, 4>::from("1RB 0RA 2RB 3RB  2LB 1LA 3LA 3LA")
            .twostep_slots(),
        Set::from([
            ((0, 2), (1, 1)),
            ((0, 2), (1, 3)),
            ((0, 3), (1, 1)),
            ((0, 3), (1, 3)),
        ]),
    );
}

/**************************************/

type Entry = (Slot, (Color, Shift));
pub type Entries = Vec<Entry>;
pub type Entrypoints = Dict<State, (Entries, Entries)>;

impl<const s: usize, const c: usize> Prog<s, c> {
    pub fn get_entrypoints(&self) -> Entrypoints {
        let mut entrypoints = Entrypoints::new();

        for (slot @ (read, _), &(color, shift, state)) in self.iter() {
            let (same, diff) = entrypoints.entry(state).or_default();

            (if read == state { same } else { diff })
                .push((slot, (color, shift)));
        }

        entrypoints
    }

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

pub trait GetInstr {
    #[expect(clippy::result_unit_err)]
    fn get_instr(&self, slot: &Slot) -> Result<Option<Instr>, ()>;
    fn is_blank(&self, color: Color) -> bool;
}

impl<const s: usize, const c: usize> GetInstr for Prog<s, c> {
    fn get_instr(&self, slot: &Slot) -> Result<Option<Instr>, ()> {
        Ok(self.get(slot).copied())
    }

    fn is_blank(&self, color: Color) -> bool {
        color == 0
    }
}

#[expect(clippy::multiple_inherent_impl)]
impl<const s: usize, const c: usize> Prog<s, c> {
    pub fn make_lru_macro(&self) -> HistoryMacro<'_, s, c> {
        HistoryMacro::new(self, update_lru)
    }

    pub fn make_transcript_macro(
        &self,
        steps: usize,
    ) -> HistoryMacro<'_, s, c> {
        HistoryMacro::new(self, update_transcript(steps))
    }
}

type History = Vec<Slot>;

pub struct HistoryMacro<'p, const s: usize, const c: usize> {
    prog: &'p Prog<s, c>,

    instrs: RefCell<Dict<Slot, Instr>>,

    colors: RefCell<Vec<(Color, History)>>,

    updater: Box<dyn Fn(Slot, &History) -> History>,
}

impl<const s: usize, const c: usize> Display
    for HistoryMacro<'_, s, c>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.prog)
    }
}

impl<const s: usize, const c: usize> GetInstr
    for HistoryMacro<'_, s, c>
{
    fn is_blank(&self, color: Color) -> bool {
        self.colors.borrow()[color as usize].0 == 0
    }

    fn get_instr(
        &self,
        macro_slot @ &(state, macro_color): &Slot,
    ) -> Result<Option<Instr>, ()> {
        if let Some(&instr) = self.instrs.borrow().get(macro_slot) {
            return Ok(Some(instr));
        }

        let (slot, updated) = {
            let (color, past) =
                &self.colors.borrow()[macro_color as usize];

            let slot = (state, *color);

            let updated = (self.updater)(slot, past);

            (slot, updated)
        };

        let Some(&(print, shift, trans)) = self.prog.get(&slot) else {
            return Ok(None);
        };

        let macro_instr = (self.encode(print, updated)?, shift, trans);

        self.instrs.borrow_mut().insert(*macro_slot, macro_instr);

        Ok(Some(macro_instr))
    }
}

impl<'p, const s: usize, const c: usize> HistoryMacro<'p, s, c> {
    fn new(
        prog: &'p Prog<s, c>,
        updater: impl 'static + Fn(Slot, &History) -> History,
    ) -> Self {
        // Macro color 0 is the fresh, blank tape cell. Subsequent colors
        // retain the complete (base color, history) pair, even if base is 0.
        let colors = vec![(0, vec![])];

        Self {
            prog,
            instrs: Dict::new().into(),
            colors: colors.into(),
            updater: Box::new(updater),
        }
    }

    fn encode(
        &self,
        print: Color,
        history: History,
    ) -> Result<Color, ()> {
        let new = (print, history);
        let mut colors = self.colors.borrow_mut();

        if let Some((index, _)) =
            colors.iter().enumerate().find(|&(_, entry)| *entry == new)
        {
            return Color::try_from(index).map_err(|_| ());
        }

        // No history is discarded when the u8 macro-color space is full.
        let index = Color::try_from(colors.len()).map_err(|_| ())?;
        colors.push(new);
        Ok(index)
    }
}

fn update_lru(slot: Slot, past: &History) -> History {
    let mut history = past.clone();

    if history.first() != Some(&slot) {
        history.retain(|&s| s != slot);
        history.insert(0, slot);
    }

    history
}

fn update_transcript(
    steps: usize,
) -> impl Fn(Slot, &History) -> History {
    move |slot, past| {
        let mut history = past.clone();

        if steps <= history.len() {
            history.pop();
        }

        history.insert(0, slot);

        history
    }
}
