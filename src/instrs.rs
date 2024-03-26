use std::collections::HashMap;

pub type Color = u64;
pub type State = u64;
pub type Shift = bool;

pub type Slot = (State, Color);
pub type Instr = (Color, Shift, State);

pub type CompThin = HashMap<Slot, Instr>;
pub type CompThic = HashMap<Slot, Option<Instr>>;
