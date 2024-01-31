use std::collections::HashMap;

pub type Color = u64;
pub type State = i32;
pub type Shift = bool;

pub type Slot = (State, Color);
pub type Instr = (Color, Shift, State);
pub type Prog = HashMap<Slot, Instr>;
