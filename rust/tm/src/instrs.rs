pub type Color = u8;
pub type State = u8;
pub type Shift = bool;

pub type Slot = (State, Color);
pub type Instr = (Color, Shift, State);

pub type Params = (State, Color);

/**************************************/

const UNDF: char = '.';

const LEFT: char = 'L';
const RIGHT: char = 'R';

/**************************************/

pub trait Parse {
    fn read(input: &str) -> Self;
    fn show(&self) -> String;
}

/**************************************/

pub const fn read_color(color: char) -> Color {
    color.to_digit(10).unwrap() as Color
}

pub const fn read_shift(shift: char) -> Shift {
    shift == RIGHT
}

pub const fn show_state(state: Option<State>) -> char {
    match state {
        None => UNDF,
        Some(s) => (s + 65) as char,
    }
}

pub fn read_state(state: char) -> State {
    State::from(state as u8 - 65)
}

impl Parse for Slot {
    fn read(slot: &str) -> Self {
        let mut chars = slot.chars();
        let state = chars.next().unwrap();
        let color = chars.next().unwrap();

        (read_state(state), read_color(color))
    }

    fn show(&self) -> String {
        let &(state, color) = self;
        format!("{}{}", show_state(Some(state)), color)
    }
}

impl Parse for Instr {
    fn read(instr: &str) -> Self {
        let mut chars = instr.chars();

        let color = chars.next().unwrap();
        let shift = chars.next().unwrap();
        let state = chars.next().unwrap();

        (read_color(color), read_shift(shift), read_state(state))
    }

    fn show(&self) -> String {
        let &(color, shift, state) = self;

        format!(
            "{}{}{}",
            color,
            if shift { RIGHT } else { LEFT },
            show_state(Some(state))
        )
    }
}

impl Parse for Option<Instr> {
    fn read(instr: &str) -> Self {
        if instr.contains(UNDF) {
            return None;
        }

        Some(Instr::read(instr))
    }

    fn show(&self) -> String {
        self.map_or_else(|| "...".to_owned(), |instr| instr.show())
    }
}

impl Parse for Option<&Instr> {
    fn read(_: &str) -> Self {
        unreachable!()
    }

    fn show(&self) -> String {
        self.map_or_else(|| "...".to_owned(), |&instr| instr.show())
    }
}

/**************************************/

#[test]
fn test_state() {
    let states = ['A', 'B', 'C'];

    for state in states {
        assert_eq!(state, show_state(Some(read_state(state))));
    }
}

#[test]
fn test_slot() {
    let slots = ["A0", "A1", "A2", "B0", "B1", "B2", "C0", "C1", "C2"];

    for slot in slots {
        assert_eq!(slot, Slot::read(slot).show());
    }
}

#[test]
fn test_instr() {
    let instrs = ["1RB", "2LC"];

    for instr in instrs {
        assert_eq!(instr, Instr::read(instr).show());
    }

    let undfnd = "...";

    assert_eq!(undfnd, Option::<Instr>::read(undfnd).show());
}
