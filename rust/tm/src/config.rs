use core::fmt;

use crate::{Parse as _, Slot, State};

pub use crate::tape::TapeLike;

/**************************************/

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Config<Tape: TapeLike> {
    pub state: State,
    pub tape: Tape,
}

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
