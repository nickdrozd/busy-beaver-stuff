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
    clippy::cast_possible_truncation,
    clippy::must_use_candidate,
    clippy::missing_panics_doc
)]
#![allow(clippy::enum_glob_use)]
#![deny(clippy::restriction)]
#![expect(
    clippy::allow_attributes_without_reason,
    clippy::arbitrary_source_item_ordering,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::blanket_clippy_restriction_lints,
    clippy::default_numeric_fallback,
    clippy::else_if_without_else,
    clippy::exhaustive_enums,
    clippy::exhaustive_structs,
    clippy::impl_trait_in_params,
    clippy::implicit_return,
    clippy::indexing_slicing,
    clippy::integer_division,
    clippy::integer_division_remainder_used,
    clippy::min_ident_chars,
    clippy::missing_assert_message,
    clippy::missing_trait_methods,
    clippy::module_name_repetitions,
    clippy::pattern_type_mismatch,
    clippy::print_stdout,
    clippy::question_mark_used,
    clippy::renamed_function_params,
    clippy::semicolon_outside_block,
    clippy::shadow_reuse,
    clippy::shadow_unrelated,
    clippy::single_call_fn,
    clippy::single_char_lifetime_names,
    clippy::std_instead_of_alloc,
    clippy::unwrap_used,
    clippy::wildcard_enum_match_arm
)]
#![allow(
    clippy::let_underscore_untyped,
    clippy::missing_docs_in_private_items,
    clippy::missing_inline_in_public_items,
    clippy::multiple_inherent_impl,
    clippy::redundant_test_prefix,
    clippy::separated_literal_suffix,
    clippy::tests_outside_test_module,
    clippy::unreachable
)]

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
