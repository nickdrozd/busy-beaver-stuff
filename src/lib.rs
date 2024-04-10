#![deny(
    clippy::restriction,
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
    clippy::if_then_some_else_none,
    clippy::unneeded_field_pattern,
    clippy::redundant_type_annotations
)]
#![expect(clippy::cast_possible_truncation)]
#![allow(clippy::enum_glob_use)]
#![expect(
    clippy::absolute_paths,
    clippy::allow_attributes_without_reason,
    clippy::arbitrary_source_item_ordering,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::blanket_clippy_restriction_lints,
    clippy::default_numeric_fallback,
    clippy::empty_enum_variants_with_brackets,
    clippy::else_if_without_else,
    clippy::impl_trait_in_params,
    clippy::implicit_return,
    clippy::indexing_slicing,
    clippy::integer_division,
    clippy::integer_division_remainder_used,
    clippy::min_ident_chars,
    clippy::missing_assert_message,
    clippy::missing_trait_methods,
    clippy::pattern_type_mismatch,
    clippy::print_stdout,
    clippy::question_mark_used,
    clippy::semicolon_outside_block,
    clippy::shadow_reuse,
    clippy::shadow_unrelated,
    clippy::single_call_fn,
    clippy::single_char_lifetime_names,
    clippy::std_instead_of_alloc,
    clippy::unimplemented,
    clippy::unwrap_used,
    clippy::wildcard_enum_match_arm
)]
#![allow(
    clippy::cfg_not_test,
    clippy::let_underscore_untyped,
    clippy::missing_docs_in_private_items,
    clippy::multiple_inherent_impl,
    clippy::redundant_pub_crate,
    clippy::redundant_test_prefix,
    clippy::should_panic_without_expect,
    clippy::tests_outside_test_module,
    clippy::unreachable
)]

mod blocks;
mod cps;
mod ctl;
mod export;
mod graph;
mod instrs;
mod machine;
mod macros;
mod prover;
mod reason;
mod rules;
mod segment;
mod tape;
mod tree;

#[cfg(test)]
mod test;

/**************************************/

use pyo3::pymodule;

#[pymodule]
mod rust_stuff {
    #[pymodule_export]
    use crate::{
        export::{
            py_cant_blank, py_cant_halt, py_cant_spin_out,
            py_cps_cant_blank, py_cps_cant_halt, py_cps_cant_spin_out,
            py_ctl_cant_blank, py_ctl_cant_halt, py_ctl_cant_spin_out,
            py_is_connected, py_opt_block, py_quick_term_or_rec,
            py_segment_cant_blank, py_segment_cant_halt,
            py_segment_cant_spin_out, py_show_comp, run_quick_machine,
            tcompile, tree_progs, BackwardResult, MachineResult,
            TermRes,
        },
        instrs::{
            read_instr, read_slot, show_instr, show_slot, show_state,
        },
        prover::PastConfigs,
    };
}
