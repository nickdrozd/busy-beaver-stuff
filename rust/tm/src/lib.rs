pub mod blocks;
pub mod cps;
pub mod ctl;
pub mod graph;
pub mod instrs;
pub mod machine;
pub mod macros;
pub mod prog;
pub mod prover;
pub mod reason;
pub mod rules;
pub mod segment;
pub mod tape;
pub mod tree;

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

/**************************************/

#[derive(Clone, Copy)]
pub enum Goal {
    Halt,
    Blank,
    Spinout,
}

impl Goal {
    pub const fn is_halt(&self) -> bool {
        matches!(self, Self::Halt)
    }

    pub const fn is_blank(&self) -> bool {
        matches!(self, Self::Blank)
    }

    pub const fn is_spinout(&self) -> bool {
        matches!(self, Self::Spinout)
    }
}
