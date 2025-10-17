use core::fmt::{self, Display};

use std::borrow::Cow;

use crate::{
    Parse as _, Slot, State,
    tape::{BigTape, Init, LilTape, MedTape},
};

pub use crate::tape::Scan;

pub type LilConfig = Config<LilTape>;
pub type MedConfig = Config<MedTape>;
pub type BigConfig = Config<BigTape>;

pub type PassConfig<'c> = Cow<'c, MedConfig>;

/**************************************/

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Config<Tape> {
    pub state: State,
    pub tape: Tape,
}

impl<Tape: Init> Config<Tape> {
    pub fn init() -> Self {
        Self {
            state: 0,
            tape: Tape::init(),
        }
    }

    pub fn init_stepped() -> Self {
        Self {
            state: 1,
            tape: Tape::init_stepped(),
        }
    }
}

/**************************************/

impl<Tape: Scan> Config<Tape> {
    pub fn slot(&self) -> Slot {
        (self.state, self.tape.scan())
    }
}

impl<Tape: Scan + Display> Display for Config<Tape> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let tape = &self.tape;
        let slot = self.slot().show();

        write!(f, "{slot} | {tape}")
    }
}
