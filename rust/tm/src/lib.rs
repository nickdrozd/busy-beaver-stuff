#![deny(
    clippy::all,
    clippy::panic,
    clippy::nursery,
    clippy::pedantic,
    clippy::get_unwrap,
    clippy::ref_patterns,
    clippy::str_to_string,
    clippy::clone_on_ref_ptr,
    clippy::same_name_method,
    clippy::allow_attributes,
    clippy::unwrap_in_result,
    clippy::partial_pub_fields,
    clippy::unused_trait_names,
    clippy::std_instead_of_core,
    mismatched_lifetime_syntaxes,
    clippy::if_then_some_else_none,
    clippy::unneeded_field_pattern,
    clippy::redundant_type_annotations,
    clippy::decimal_literal_representation
)]
#![expect(
    clippy::trivially_copy_pass_by_ref,
    clippy::must_use_candidate,
    clippy::missing_panics_doc
)]
#![allow(clippy::enum_glob_use)]

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
