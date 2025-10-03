use core::fmt;

use std::borrow::Cow;

use crate::{
    Parse as _, Slot, State,
    tape::{LilTape, MedTape},
};

pub use crate::tape::TapeLike;

pub type LilConfig = Config<LilTape>;
pub type MedConfig = Config<MedTape>;

pub type PassConfig<'c> = Cow<'c, MedConfig>;

/**************************************/

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Config<Tape: TapeLike> {
    pub state: State,
    pub tape: Tape,
}

impl MedConfig {
    pub fn init_stepped() -> Self {
        Self {
            state: 1,
            tape: MedTape::init_stepped(),
        }
    }
}

/**************************************/

impl<Tape: TapeLike> Config<Tape> {
    pub fn slot(&self) -> Slot {
        (self.state, self.tape.scan())
    }
}

impl<Tape: TapeLike> fmt::Display for Config<Tape> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let tape = &self.tape;
        let slot = self.slot().show();

        write!(f, "{slot} | {tape}")
    }
}
