use std::collections::BTreeMap as Dict;

pub type Color = u64;
pub type State = u64;
pub type Shift = bool;

pub type Slot = (State, Color);
pub type Instr = (Color, Shift, State);

pub type CompProg = Dict<Slot, Instr>;
