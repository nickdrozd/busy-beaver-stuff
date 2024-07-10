use crate::instrs::{CompProg, Instr, Slot};

pub trait GetInstr {
    fn get_instr(&self, slot: &Slot) -> Option<Instr>;
}

impl GetInstr for CompProg {
    fn get_instr(&self, slot: &Slot) -> Option<Instr> {
        self.get(slot).copied()
    }
}
