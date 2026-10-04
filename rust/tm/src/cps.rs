use core::{
    fmt,
    hash::{Hash, Hasher},
};
use std::collections::hash_map::Entry;

use ahash::AHashMap as Dict;

use crate::{Color, Goal, Prog, Shift, config, macros::GetInstr};

use Goal::*;

pub type Radius = usize;

const MAX_DEPTH: usize = 10_000;

/**************************************/

impl<const s: usize, const c: usize> Prog<s, c> {
    pub fn cps_cant_halt(&self, rad: Radius) -> bool {
        self.cps_run_macros(rad, Halt)
    }

    pub fn cps_cant_blank(&self, rad: Radius) -> bool {
        self.cps_run_macros(rad, Blank)
    }

    pub fn cps_cant_spinout(&self, rad: Radius) -> bool {
        self.cps_run_macros(rad, Spinout)
    }

    fn cps_run_macros(&self, rad: Radius, goal: Goal) -> bool {
        assert!(rad > 1);

        let mut configs = Configs::new(goal);

        (2..rad).any(|seg| {
            cps_cant_reach(self, seg, goal, &mut configs)
                || [1, 4, 16].iter().any(|tr| {
                    cps_cant_reach(
                        &self.make_transcript_macro(*tr),
                        seg,
                        goal,
                        &mut configs,
                    )
                })
                || cps_cant_reach(
                    &self.make_lru_macro(),
                    seg,
                    goal,
                    &mut configs,
                )
        })
    }
}

fn cps_cant_reach(
    prog: &impl GetInstr,
    rad: Radius,
    goal: Goal,
    configs: &mut Configs,
) -> bool {
    match goal {
        Halt => {
            cps_cant_reach_goal::<CPS_GOAL_HALT>(prog, rad, configs)
        },
        Blank => {
            cps_cant_reach_goal::<CPS_GOAL_BLANK>(prog, rad, configs)
        },
        Spinout => {
            cps_cant_reach_goal::<CPS_GOAL_SPINOUT>(prog, rad, configs)
        },
    }
}

#[inline]
fn cps_cant_reach_goal<const G: u8>(
    prog: &impl GetInstr,
    rad: Radius,
    configs: &mut Configs,
) -> bool {
    macro_rules! try_level {
        ($level:literal) => {
            match cps_cant_reach_level::<G, $level>(prog, rad, configs)
            {
                CpsOutcome::Proved => return true,
                CpsOutcome::Counterexample => {},
                CpsOutcome::Inconclusive => return false,
            }
        };
    }

    try_level!(0);
    if !TAIL_SIG_REFINEMENTS.is_empty() {
        try_level!(1);
    }
    if TAIL_SIG_REFINEMENTS.len() >= 2 {
        try_level!(2);
    }

    false
}

/**************************************/

const CPS_GOAL_HALT: u8 = 0;
const CPS_GOAL_BLANK: u8 = 1;
const CPS_GOAL_SPINOUT: u8 = 2;

const MAX_NONZERO_COUNT: NonzeroCount = 0x7fff;

const fn parity_lower_bound(
    nz: NonzeroCount,
    odd: bool,
) -> NonzeroCount {
    let nz = if nz > MAX_NONZERO_COUNT {
        MAX_NONZERO_COUNT
    } else {
        nz
    };
    let nz_odd = nz & 1 != 0;
    if nz_odd == odd {
        nz
    } else if nz == MAX_NONZERO_COUNT {
        // At saturation, step down to the largest representable lower bound
        // with the required parity. This only weakens the lower bound.
        nz - 1
    } else {
        nz + 1
    }
}

#[expect(clippy::cognitive_complexity)]
fn cps_cant_reach_level<const G: u8, const LEVEL: usize>(
    prog: &impl GetInstr,
    rad: Radius,
    configs: &mut Configs,
) -> CpsOutcome {
    debug_assert!(G <= CPS_GOAL_SPINOUT);
    debug_assert!(LEVEL <= TAIL_SIG_REFINEMENTS.len());
    configs.reset::<G>(rad);

    while configs.todo_head < configs.todo.len() {
        let config_id = configs.todo[configs.todo_head];
        configs.todo_head += 1;
        if !configs.is_active::<G>(config_id) {
            continue;
        }

        let (state, mut tape) = {
            let config = &configs.by_id[id_index(config_id)];
            (config.state, config.tape)
        };

        let init_scan = tape.scan;

        let (print, shift, next_state) =
            match prog.get_instr(&(state, init_scan)) {
                Err(_) => return CpsOutcome::Inconclusive,
                Ok(None) => {
                    if G == CPS_GOAL_HALT {
                        return CpsOutcome::Counterexample;
                    }
                    continue;
                },
                Ok(Some(instr)) => instr,
            };

        // The summary on the pushed side describes the hidden tail beyond
        // push.last.  Record it together with that continuation color so a
        // later pull cannot combine the color from one occurrence with the
        // tail evidence from another.
        let push_tail_nz =
            if shift { tape.left_nz } else { tape.right_nz };
        let push_tail_sig = if shift {
            tape.left_sig()
        } else {
            tape.right_sig()
        };
        let push_tail_parity = if G == CPS_GOAL_HALT {
            false
        } else if shift {
            tape.left_parity()
        } else {
            tape.right_parity()
        };

        // Keep the mutable borrows of the two span fields local.  Span is
        // Copy, so after the push/pull transition we retain value snapshots
        // instead of references into `tape`.  This lets the packed metadata
        // accessors mutate/read `tape` freely below.
        let dropped = if shift {
            configs.add_span::<G>(
                shift,
                &tape.lspan,
                push_tail_nz,
                push_tail_parity,
                push_tail_sig,
            );
            let dropped =
                tape.lspan.push(print, &mut configs.span_pool);
            tape.scan = tape.rspan.pull(&mut configs.span_pool);
            dropped
        } else {
            configs.add_span::<G>(
                shift,
                &tape.rspan,
                push_tail_nz,
                push_tail_parity,
                push_tail_sig,
            );
            let dropped =
                tape.rspan.push(print, &mut configs.span_pool);
            tape.scan = tape.lspan.pull(&mut configs.span_pool);
            dropped
        };

        let pull = if shift { tape.rspan } else { tape.lspan };
        let push = if shift { tape.lspan } else { tape.rspan };

        if shift {
            let next_sig = tape.left_sig().prepend::<LEVEL>(dropped);
            tape.set_left_sig(next_sig);
        } else {
            let next_sig = tape.right_sig().prepend::<LEVEL>(dropped);
            tape.set_right_sig(next_sig);
        }

        if G != CPS_GOAL_HALT {
            let dropped_nz = if G == CPS_GOAL_BLANK {
                !prog.is_blank(dropped)
            } else {
                dropped != 0
            };

            if dropped_nz {
                if shift {
                    tape.toggle_left_parity();
                    tape.left_nz = parity_lower_bound(
                        tape.left_nz.saturating_add(1),
                        tape.left_parity(),
                    );
                } else {
                    tape.toggle_right_parity();
                    tape.right_nz = parity_lower_bound(
                        tape.right_nz.saturating_add(1),
                        tape.right_parity(),
                    );
                }
            }
        }

        // These whole-window predicates do not depend on which
        // continuation enters the represented window.  Computing them once
        // avoids rescanning both spans for every continuation.
        let blank_window = if G == CPS_GOAL_BLANK {
            prog.is_blank(tape.scan)
                && pull.base_blank_span(prog, &mut configs.span_pool)
                && push.base_all_blank(prog, &mut configs.span_pool)
        } else if G == CPS_GOAL_SPINOUT {
            init_scan == 0
                && tape.scan == 0
                && pull.blank_span(&configs.span_pool)
                && state == next_state
        } else {
            false
        };

        let Configs {
            lspans,
            rspans,
            interner,
            by_id,
            active,
            active_count,
            todo,
            continuation_cursor,
            l_watch,
            r_watch,
            ..
        } = &mut *configs;

        macro_rules! process_continuation {
            (
                $color:expr,
                $tail_nz:expr,
                $tail_parity:expr,
                $tail_sig:expr
            ) => {{
                let color = $color;
                let tail_nz = $tail_nz;
                let tail_parity = $tail_parity;
                let tail_sig = $tail_sig;

                let current_sig = if shift {
                    tape.right_sig()
                } else {
                    tape.left_sig()
                };
                if !current_sig.is_prepend_of::<LEVEL>(color, tail_sig)
                {
                    continue;
                }

                let (left_sig, right_sig) = if shift {
                    (tape.left_sig(), tail_sig)
                } else {
                    (tail_sig, tape.right_sig())
                };

                let (left_nz, right_nz, left_parity, right_parity) =
                    if G == CPS_GOAL_HALT {
                        (0, 0, false, false)
                    } else {
                        let entered_nz = if G == CPS_GOAL_BLANK {
                            !prog.is_blank(color)
                        } else {
                            color != 0
                        };

                        let current_parity = if shift {
                            tape.right_parity()
                        } else {
                            tape.left_parity()
                        };

                        if current_parity != (entered_nz ^ tail_parity)
                        {
                            continue;
                        }

                        let current_nz = if shift {
                            tape.right_nz
                        } else {
                            tape.left_nz
                        };
                        let current_tail_nz = parity_lower_bound(
                            current_nz,
                            current_parity,
                        )
                        .saturating_sub(entered_nz.into());
                        let continuation_tail_nz =
                            parity_lower_bound(tail_nz, tail_parity);
                        let next_tail_nz =
                            current_tail_nz.max(continuation_tail_nz);

                        if shift {
                            (
                                tape.left_nz,
                                next_tail_nz,
                                tape.left_parity(),
                                tail_parity,
                            )
                        } else {
                            (
                                next_tail_nz,
                                tape.right_nz,
                                tail_parity,
                                tape.right_parity(),
                            )
                        }
                    };

                let reached_goal = if G == CPS_GOAL_BLANK {
                    blank_window
                        && prog.is_blank(color)
                        && left_nz == 0
                        && right_nz == 0
                        && !left_parity
                        && !right_parity
                } else if G == CPS_GOAL_SPINOUT {
                    let (ahead_nz, ahead_parity, ahead_sig) = if shift {
                        (right_nz, right_parity, right_sig)
                    } else {
                        (left_nz, left_parity, left_sig)
                    };

                    blank_window
                        && color == 0
                        && ahead_nz == 0
                        && !ahead_parity
                        && ahead_sig == TailSig::default()
                } else {
                    false
                };

                if reached_goal {
                    return CpsOutcome::Counterexample;
                }

                let mut pull_clone = pull;
                pull_clone.set_last(color);

                let next_tape = Tape::from_spans(
                    tape.scan,
                    push,
                    pull_clone,
                    shift,
                    left_nz,
                    right_nz,
                    left_parity,
                    right_parity,
                    left_sig,
                    right_sig,
                );

                let next_config = Config {
                    state: next_state,
                    tape: next_tape,
                };

                let (next_id, is_new) = interner.intern::<G>(
                    next_config,
                    by_id,
                    active,
                    active_count,
                );

                if is_new {
                    debug_assert_eq!(
                        continuation_cursor.len(),
                        id_index(next_id)
                    );
                    continuation_cursor
                        .push(UNSEEN_CONTINUATION_CURSOR);
                    todo.push(next_id);
                }
            }};
        }

        let pull_key = pull.span();
        let cursor = continuation_cursor[id_index(config_id)];
        let pull_spans = if shift { &*rspans } else { &*lspans };
        let next_cursor = pull_spans.delta_len::<G>(&pull);
        let continuations: &[Continuation] =
            if cursor == UNSEEN_CONTINUATION_CURSOR {
                // First processing of this configuration: consume only the
                // current canonical set, not obsolete historical Rich values.
                pull_spans.current_continuations::<G>(&pull)
            } else {
                // Wakeup: consume only facts learned since the last time this
                // config drained this span's event stream.
                pull_spans.continuation_deltas::<G>(&pull, cursor)
            };

        for &continuation in continuations {
            process_continuation!(
                continuation.color(),
                continuation.tail_nz(),
                continuation.tail_parity(),
                continuation.tail_sig()
            );
        }

        continuation_cursor[id_index(config_id)] = next_cursor;

        let config_is_active = if G == CPS_GOAL_HALT {
            true
        } else {
            active.get(id_index(config_id)).copied().unwrap_or(false)
        };

        if config_is_active {
            let watch = if shift { r_watch } else { l_watch };

            let pull_idx = id_index(pull_key);
            if watch.len() <= pull_idx {
                watch.resize_with(pull_idx + 1, Vec::new);
            }
            watch[pull_idx].push(config_id);
        }

        if ConfigInterner::at_capacity::<G>(by_id.len(), *active_count)
        {
            return CpsOutcome::Inconclusive;
        }
    }

    CpsOutcome::Proved
}

/**************************************/

/// Hidden-tail predicates added after a coarse CPS counterexample.  Level 0
/// uses no signatures; level n uses the first n modular channels.  Each
/// channel is an exact homomorphism of a finite-support ray, so collisions
/// can only preserve spurious behavior, never remove concrete behavior.
/// Entries are `(polynomial_base, modulus)`.
const TAIL_SIG_REFINEMENTS: [(u8, u8); 2] = [(2, 3), (3, 4)];

const _: () = {
    assert!(TAIL_SIG_REFINEMENTS.len() * 3 <= 8);
    let mut i = 0;
    while i < TAIL_SIG_REFINEMENTS.len() {
        let modulus = TAIL_SIG_REFINEMENTS[i].1;
        assert!(modulus > 0 && modulus <= 8);
        i += 1;
    }
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum CpsOutcome {
    Proved,
    Counterexample,
    Inconclusive,
}

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
struct TailSig(u8);

#[expect(clippy::cast_possible_truncation)]
impl TailSig {
    const BITS_PER_CHANNEL: usize = 3;
    const CHANNEL_MASK: u8 = (1 << Self::BITS_PER_CHANNEL) - 1;
    const USED_BITS: usize =
        Self::BITS_PER_CHANNEL * TAIL_SIG_REFINEMENTS.len();
    const PACKED_MASK: u8 = if Self::USED_BITS == 8 {
        u8::MAX
    } else {
        ((1_u16 << Self::USED_BITS) - 1) as u8
    };

    const fn raw(self) -> u8 {
        self.0
    }

    #[inline]
    fn prepend<const LEVEL: usize>(self, color: Color) -> Self {
        debug_assert!(LEVEL <= TAIL_SIG_REFINEMENTS.len());

        let color = u16::from(color);
        let mut packed = self.0;

        if LEVEL >= 1 {
            let (base, modulus) = TAIL_SIG_REFINEMENTS[0];
            let old = packed & Self::CHANNEL_MASK;
            let next = (color + u16::from(base) * u16::from(old))
                % u16::from(modulus);
            packed = (packed & !Self::CHANNEL_MASK) | next as u8;
        }

        if LEVEL >= 2 {
            let (base, modulus) = TAIL_SIG_REFINEMENTS[1];
            let shift = Self::BITS_PER_CHANNEL;
            let old = (packed >> shift) & Self::CHANNEL_MASK;
            let next = (color + u16::from(base) * u16::from(old))
                % u16::from(modulus);
            let mask = Self::CHANNEL_MASK << shift;
            packed = (packed & !mask) | ((next as u8) << shift);
        }

        Self(packed)
    }

    #[inline]
    fn is_prepend_of<const LEVEL: usize>(
        self,
        color: Color,
        tail: Self,
    ) -> bool {
        self == tail.prepend::<LEVEL>(color)
    }
}

type SpanId = u32;

type Colors = Vec<Color>;

#[derive(Clone, Copy, PartialEq, Eq)]
struct Continuation(u32);

#[expect(clippy::cast_possible_truncation)]
impl Continuation {
    //  0..=7   color
    //  8..=22  compact nonzero lower bound (15 bits)
    //  23       exact parity
    // 24..=31  packed tail signature
    fn new(
        color: Color,
        tail_nz: NonzeroCount,
        tail_parity: bool,
        tail_sig: TailSig,
    ) -> Self {
        debug_assert!(tail_nz <= MAX_NONZERO_COUNT);
        Self(
            u32::from(color)
                | (u32::from(tail_nz) << 8)
                | (u32::from(tail_parity) << 23)
                | (u32::from(tail_sig.raw()) << 24),
        )
    }

    const fn color(self) -> Color {
        self.0 as Color
    }

    const fn tail_nz(self) -> NonzeroCount {
        ((self.0 >> 8) & 0x7fff) as NonzeroCount
    }

    const fn tail_parity(self) -> bool {
        self.0 & (1 << 23) != 0
    }

    const fn tail_sig(self) -> TailSig {
        TailSig((self.0 >> 24) as u8)
    }

    fn set_tail_nz(&mut self, tail_nz: NonzeroCount) {
        debug_assert!(tail_nz <= MAX_NONZERO_COUNT);
        self.0 = (self.0 & !(0x7fff << 8)) | (u32::from(tail_nz) << 8);
    }
}

type Continuations = Vec<Continuation>;

#[derive(Default)]
struct SpanContinuations {
    // Canonical continuation set used when a configuration first reaches
    // this span. Rich CPS keeps only the smallest tail_nz for each key.
    current: Continuations,
    // Append-only changes to `current`. A waiting configuration remembers
    // how far it consumed this stream, so wakeups process only new facts.
    deltas: Continuations,
}

type RichSpans = Vec<SpanContinuations>;
type HaltSpans = Vec<SpanContinuations>;

enum Spans {
    Halt(HaltSpans),
    Rich(RichSpans),
}

type ConfigId = u32;
type NonzeroCount = u16;
type ContinuationCursor = u32;
type Watch = Vec<Vec<ConfigId>>;

const UNSEEN_CONTINUATION_CURSOR: ContinuationCursor =
    ContinuationCursor::MAX;

fn id_index(id: u32) -> usize {
    usize::try_from(id).expect("u32 CPS index must fit usize")
}

#[inline]
fn compact_id(index: usize) -> u32 {
    u32::try_from(index).expect("CPS table exceeded u32 index space")
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct ConfigShape([u32; 3]);

impl From<&Config> for ConfigShape {
    fn from(config: &Config) -> Self {
        Self([
            config.tape.lspan.raw(),
            config.tape.rspan.raw(),
            u32::from(config.tape.tail_meta)
                | (u32::from(config.state) << 16)
                | (u32::from(config.tape.scan) << 24),
        ])
    }
}

impl Hash for ConfigShape {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(
            u64::from(self.0[0]) | (u64::from(self.0[1]) << 32),
        );
        state.write_u32(self.0[2]);
    }
}

#[derive(Clone, Copy)]
struct AntichainEntry(u64);

#[expect(clippy::cast_possible_truncation)]
impl AntichainEntry {
    const COUNT_MASK: u64 = 0x7fff;

    fn new(
        left_nz: NonzeroCount,
        right_nz: NonzeroCount,
        id: ConfigId,
    ) -> Self {
        Self(
            u64::from(left_nz)
                | (u64::from(right_nz) << 15)
                | (u64::from(id) << 30),
        )
    }

    const fn left_nz(self) -> NonzeroCount {
        (self.0 & Self::COUNT_MASK) as NonzeroCount
    }

    const fn right_nz(self) -> NonzeroCount {
        ((self.0 >> 15) & Self::COUNT_MASK) as NonzeroCount
    }

    const fn id(self) -> ConfigId {
        (self.0 >> 30) as ConfigId
    }
}

#[derive(Clone, Copy)]
struct AntichainEntries {
    first: u64,
    second: u64,
}

#[expect(clippy::cast_possible_truncation)]
impl AntichainEntries {
    const EMPTY: u64 = u64::MAX;
    const OVERFLOW_TAG: u64 = 1_u64 << 63;

    const fn empty() -> Self {
        Self {
            first: Self::EMPTY,
            second: Self::EMPTY,
        }
    }

    const fn overflow_index(self) -> Option<u32> {
        if self.second != Self::EMPTY
            && self.second & Self::OVERFLOW_TAG != 0
        {
            Some(self.second as u32)
        } else {
            None
        }
    }

    #[inline]
    fn find_subsuming(
        self,
        left_nz: NonzeroCount,
        right_nz: NonzeroCount,
        overflow: &[Vec<AntichainEntry>],
    ) -> Option<AntichainEntry> {
        if let Some(index) = self.overflow_index() {
            return overflow[id_index(index)].iter().copied().find(
                |entry| {
                    entry.left_nz() <= left_nz
                        && entry.right_nz() <= right_nz
                },
            );
        }

        [self.first, self.second]
            .into_iter()
            .filter(|&raw| raw != Self::EMPTY)
            .map(AntichainEntry)
            .find(|entry| {
                entry.left_nz() <= left_nz
                    && entry.right_nz() <= right_nz
            })
    }

    fn retire_dominated(
        &mut self,
        left_nz: NonzeroCount,
        right_nz: NonzeroCount,
        overflow: &mut [Vec<AntichainEntry>],
        active: &mut [bool],
        active_count: &mut usize,
    ) {
        let mut retire = |entry: AntichainEntry| {
            let dominated = left_nz <= entry.left_nz()
                && right_nz <= entry.right_nz();
            if dominated {
                let idx = id_index(entry.id());
                debug_assert!(active[idx]);
                active[idx] = false;
                *active_count -= 1;
            }
            dominated
        };

        if let Some(index) = self.overflow_index() {
            overflow[id_index(index)].retain(|&entry| !retire(entry));
            return;
        }

        let mut kept = [Self::EMPTY; 2];
        let mut len = 0;
        for raw in [self.first, self.second] {
            if raw == Self::EMPTY {
                continue;
            }
            let entry = AntichainEntry(raw);
            if !retire(entry) {
                kept[len] = raw;
                len += 1;
            }
        }
        self.first = kept[0];
        self.second = kept[1];
    }

    fn push(
        &mut self,
        entry: AntichainEntry,
        overflow: &mut Vec<Vec<AntichainEntry>>,
        free_overflow: &mut Vec<u32>,
    ) {
        if let Some(index) = self.overflow_index() {
            overflow[id_index(index)].push(entry);
            return;
        }

        if self.first == Self::EMPTY {
            self.first = entry.0;
            return;
        }
        if self.second == Self::EMPTY {
            self.second = entry.0;
            return;
        }

        let index = if let Some(index) = free_overflow.pop() {
            overflow[id_index(index)].clear();
            index
        } else {
            let index = compact_id(overflow.len());
            overflow.push(Vec::new());
            index
        };

        let entries = &mut overflow[id_index(index)];
        entries.push(AntichainEntry(self.first));
        entries.push(AntichainEntry(self.second));
        entries.push(entry);
        self.first = Self::EMPTY;
        self.second = Self::OVERFLOW_TAG | u64::from(index);
    }
}

enum ConfigInterner {
    Exact(Dict<ConfigShape, ConfigId>),
    Antichain {
        seen: Dict<ConfigShape, AntichainEntries>,
        overflow: Vec<Vec<AntichainEntry>>,
        free_overflow: Vec<u32>,
    },
}

impl ConfigInterner {
    #[inline]
    fn intern<const G: u8>(
        &mut self,
        config: Config,
        by_id: &mut Vec<Config>,
        active: &mut Vec<bool>,
        active_count: &mut usize,
    ) -> (ConfigId, bool) {
        if G == CPS_GOAL_HALT {
            let Self::Exact(seen) = self else {
                unreachable!("halt CPS must use exact interning")
            };

            debug_assert_eq!(config.tape.left_nz, 0);
            debug_assert_eq!(config.tape.right_nz, 0);
            debug_assert!(!config.tape.left_parity());
            debug_assert!(!config.tape.right_parity());

            let shape = ConfigShape::from(&config);
            match seen.entry(shape) {
                Entry::Occupied(entry) => (*entry.get(), false),
                Entry::Vacant(entry) => {
                    let next_id = compact_id(by_id.len());
                    by_id.push(config);
                    entry.insert(next_id);
                    (next_id, true)
                },
            }
        } else {
            let Self::Antichain {
                seen,
                overflow,
                free_overflow,
            } = self
            else {
                unreachable!("rich CPS must use antichain interning")
            };

            let shape = ConfigShape::from(&config);
            let left_nz = parity_lower_bound(
                config.tape.left_nz,
                config.tape.left_parity(),
            );
            let right_nz = parity_lower_bound(
                config.tape.right_nz,
                config.tape.right_parity(),
            );
            let entries = match seen.entry(shape) {
                Entry::Occupied(entry) => entry.into_mut(),
                Entry::Vacant(entry) => {
                    entry.insert(AntichainEntries::empty())
                },
            };

            if let Some(entry) =
                entries.find_subsuming(left_nz, right_nz, overflow)
            {
                return (entry.id(), false);
            }

            entries.retire_dominated(
                left_nz,
                right_nz,
                overflow,
                active,
                active_count,
            );

            let next_id = compact_id(active.len());
            debug_assert_eq!(by_id.len(), id_index(next_id));
            by_id.push(config);
            active.push(true);
            *active_count += 1;
            entries.push(
                AntichainEntry::new(left_nz, right_nz, next_id),
                overflow,
                free_overflow,
            );

            (next_id, true)
        }
    }

    fn clear(&mut self) {
        match self {
            Self::Exact(seen) => seen.clear(),
            Self::Antichain {
                seen,
                overflow,
                free_overflow,
            } => {
                for entries in seen.values().copied() {
                    if let Some(index) = entries.overflow_index() {
                        overflow[id_index(index)].clear();
                        free_overflow.push(index);
                    }
                }
                seen.clear();
            },
        }
    }

    #[inline]
    const fn at_capacity<const G: u8>(
        by_id_len: usize,
        active_count: usize,
    ) -> bool {
        let count = if G == CPS_GOAL_HALT {
            by_id_len
        } else {
            active_count
        };
        MAX_DEPTH < count
    }
}

/**************************************/

#[derive(Clone, Copy)]
struct PushTransition(u64);

#[expect(clippy::cast_possible_truncation)]
impl PushTransition {
    fn new(last: Color, color: Color, next: Span) -> Self {
        Self(
            u64::from(last)
                | (u64::from(color) << 8)
                | (u64::from(next.raw()) << 16),
        )
    }

    fn matches(self, last: Color, color: Color) -> bool {
        self.0 as u16 == (u16::from(last) | (u16::from(color) << 8))
    }

    const fn next(self) -> Span {
        Span::from_raw((self.0 >> 16) as u32)
    }
}

#[derive(Clone, Copy)]
struct PullTransition(u64);

#[expect(clippy::cast_possible_truncation)]
impl PullTransition {
    fn new(last: Color, next: Span, pulled: Color) -> Self {
        Self(
            u64::from(last)
                | (u64::from(pulled) << 8)
                | (u64::from(next.raw()) << 16),
        )
    }

    const fn last(self) -> Color {
        self.0 as Color
    }

    const fn next(self) -> Span {
        Span::from_raw((self.0 >> 16) as u32)
    }

    const fn pulled(self) -> Color {
        (self.0 >> 8) as Color
    }
}

struct SpanPool {
    spans: Vec<Colors>,
    span_count: usize,
    index: Dict<Colors, SpanId>,
    blank: Option<Vec<bool>>,
    base_blank: Option<Vec<Option<bool>>>,

    push_cache: Vec<Vec<PushTransition>>,
    pull_cache: Vec<Vec<PullTransition>>,
    spare_colors: Vec<Colors>,
}

impl SpanPool {
    fn new(goal: Goal) -> Self {
        Self {
            spans: vec![],
            span_count: 0,
            index: Dict::new(),
            blank: match goal {
                Spinout => Some(vec![]),
                Halt | Blank => None,
            },
            base_blank: match goal {
                Blank => Some(vec![]),
                Halt | Spinout => None,
            },
            push_cache: Vec::new(),
            pull_cache: Vec::new(),
            spare_colors: Vec::new(),
        }
    }

    fn reset(&mut self) {
        let old_span_count = self.span_count;

        // Only IDs used by the immediately previous run can contain live
        // transitions. Entries above that logical prefix were already
        // cleared when they last belonged to a run.
        for transitions in &mut self.push_cache[..old_span_count] {
            transitions.clear();
        }
        for transitions in &mut self.pull_cache[..old_span_count] {
            transitions.clear();
        }

        self.span_count = 0;

        // The index owns one color vector per interned span.  Retain those
        // allocations for candidate spans in later CPS runs instead of
        // dropping them when the logical span table is reset.
        for (mut colors, _) in self.index.drain() {
            colors.clear();
            self.spare_colors.push(colors);
        }

        if let Some(blank) = &mut self.blank {
            blank.clear();
        }
        if let Some(base_blank) = &mut self.base_blank {
            base_blank.clear();
        }
    }

    fn take_colors(&mut self, len: usize) -> Colors {
        let mut colors = self
            .spare_colors
            .pop()
            .unwrap_or_else(|| Vec::with_capacity(len));
        colors.clear();

        if colors.capacity() < len {
            colors.reserve(len - colors.capacity());
        }

        colors
    }

    fn recycle_colors(&mut self, mut colors: Colors) {
        colors.clear();
        self.spare_colors.push(colors);
    }

    fn intern(&mut self, colors: Colors) -> SpanId {
        if let Some(&id) = self.index.get(&colors) {
            self.recycle_colors(colors);
            return id;
        }

        let id = compact_id(self.span_count);
        assert!(
            id <= Span::ID_MASK,
            "CPS span table exceeded packed span id space"
        );
        self.span_count += 1;

        if id_index(id) == self.spans.len() {
            let mut stored = self.take_colors(colors.len());
            stored.extend_from_slice(&colors);
            self.spans.push(stored);
            self.push_cache.push(Vec::new());
            self.pull_cache.push(Vec::new());
        } else {
            let stored = &mut self.spans[id_index(id)];
            stored.clear();
            stored.extend_from_slice(&colors);
        }
        self.index.insert(colors, id);

        if let Some(blank) = &mut self.blank {
            blank.push(
                self.spans[id_index(id)]
                    .iter()
                    .all(|&color| color == 0),
            );
        }
        if let Some(base_blank) = &mut self.base_blank {
            base_blank.push(None);
        }

        id
    }

    fn colors(&self, id: SpanId) -> &Colors {
        debug_assert!(id_index(id) < self.span_count);
        &self.spans[id_index(id)]
    }

    fn blank_span(&self, id: SpanId) -> bool {
        self.blank
            .as_ref()
            .expect("canonical blank cache is only used for Spinout")
            [id_index(id)]
    }

    fn base_blank_span(
        &mut self,
        prog: &impl GetInstr,
        id: SpanId,
    ) -> bool {
        if let Some(blank) = self
            .base_blank
            .as_ref()
            .expect("base blank cache is only used for Blank")
            [id_index(id)]
        {
            return blank;
        }

        let blank = self.spans[id_index(id)]
            .iter()
            .all(|&color| prog.is_blank(color));
        self.base_blank
            .as_mut()
            .expect("base blank cache is only used for Blank")
            [id_index(id)] = Some(blank);
        blank
    }
}

/**************************************/

struct Configs {
    span_pool: SpanPool,

    lspans: Spans,
    rspans: Spans,

    interner: ConfigInterner,
    by_id: Vec<Config>,
    active: Vec<bool>,
    active_count: usize,
    todo: Vec<ConfigId>,
    todo_head: usize,
    // Per-config offset into the delta stream of the span it waits on.
    // ContinuationCursor::MAX means the config has never processed that span, so its
    // first visit consumes the canonical continuation set instead.
    continuation_cursor: Vec<ContinuationCursor>,

    l_watch: Watch,
    r_watch: Watch,
}

impl Configs {
    fn new(goal: Goal) -> Self {
        Self {
            span_pool: SpanPool::new(goal),
            lspans: Spans::new(goal),
            rspans: Spans::new(goal),
            interner: if matches!(goal, Halt) {
                ConfigInterner::Exact(Dict::new())
            } else {
                ConfigInterner::Antichain {
                    seen: Dict::new(),
                    overflow: Vec::new(),
                    free_overflow: Vec::new(),
                }
            },
            by_id: Vec::new(),
            active: Vec::new(),
            active_count: 0,
            todo: Vec::new(),
            todo_head: 0,
            continuation_cursor: Vec::new(),
            l_watch: Vec::new(),
            r_watch: Vec::new(),
        }
    }

    fn reset<const G: u8>(&mut self, rad: Radius) {
        let old_span_count = self.span_pool.span_count;

        // Span IDs are reassigned from zero. Only the prefix used by the
        // previous run can contain live continuation or watcher entries.
        self.lspans.clear(old_span_count);
        self.rspans.clear(old_span_count);

        let l_watch_count = old_span_count.min(self.l_watch.len());
        for waiting in &mut self.l_watch[..l_watch_count] {
            waiting.clear();
        }

        let r_watch_count = old_span_count.min(self.r_watch.len());
        for waiting in &mut self.r_watch[..r_watch_count] {
            waiting.clear();
        }

        self.span_pool.reset();
        self.interner.clear();
        self.by_id.clear();
        self.active.clear();
        self.active_count = 0;
        self.todo.clear();
        self.todo_head = 0;
        self.continuation_cursor.clear();

        let init = Config::init(rad, &mut self.span_pool);

        self.lspans.add_span::<G>(
            &init.tape.lspan,
            init.tape.left_nz,
            init.tape.left_parity(),
            init.tape.left_sig(),
        );
        self.rspans.add_span::<G>(
            &init.tape.rspan,
            init.tape.right_nz,
            init.tape.right_parity(),
            init.tape.right_sig(),
        );

        let (init_id, is_new) = self.intern_config::<G>(init);
        assert!(is_new);
        debug_assert_eq!(init_id, 0);
        self.todo.push(init_id);
    }

    fn intern_config<const G: u8>(
        &mut self,
        config: Config,
    ) -> (ConfigId, bool) {
        let (id, is_new) = self.interner.intern::<G>(
            config,
            &mut self.by_id,
            &mut self.active,
            &mut self.active_count,
        );

        if is_new {
            debug_assert_eq!(
                self.continuation_cursor.len(),
                id_index(id)
            );
            self.continuation_cursor.push(UNSEEN_CONTINUATION_CURSOR);
        }

        (id, is_new)
    }

    #[inline]
    fn is_active<const G: u8>(&self, id: ConfigId) -> bool {
        if G == CPS_GOAL_HALT {
            true
        } else {
            self.active.get(id_index(id)).copied().unwrap_or(false)
        }
    }

    #[inline]
    fn add_span<const G: u8>(
        &mut self,
        shift: Shift,
        span: &Span,
        tail_nz: NonzeroCount,
        tail_parity: bool,
        tail_sig: TailSig,
    ) {
        let (spans, watch) = if shift {
            (&mut self.lspans, &mut self.l_watch)
        } else {
            (&mut self.rspans, &mut self.r_watch)
        };

        if spans.add_span::<G>(span, tail_nz, tail_parity, tail_sig)
            && let Some(waiting) = watch.get_mut(id_index(span.span()))
        {
            if G == CPS_GOAL_HALT {
                self.todo.append(waiting);
            } else {
                let active = &self.active;
                self.todo.extend(waiting.drain(..).filter(|id| {
                    active.get(id_index(*id)) == Some(&true)
                }));
            }
        }
    }
}

/**************************************/

impl Spans {
    const fn new(goal: Goal) -> Self {
        match goal {
            Halt => Self::Halt(Vec::new()),
            Blank | Spinout => Self::Rich(Vec::new()),
        }
    }

    fn clear(&mut self, span_count: usize) {
        match self {
            Self::Halt(spans) | Self::Rich(spans) => {
                for span in spans.iter_mut().take(span_count) {
                    span.current.clear();
                    span.deltas.clear();
                }
            },
        }
    }

    #[inline]
    fn add_span<const G: u8>(
        &mut self,
        span: &Span,
        tail_nz: NonzeroCount,
        tail_parity: bool,
        tail_sig: TailSig,
    ) -> bool {
        if G == CPS_GOAL_HALT {
            let Self::Halt(spans) = self else {
                unreachable!(
                    "halt CPS must use halt continuation tables"
                )
            };

            debug_assert_eq!(tail_nz, 0);
            debug_assert!(!tail_parity);
            let continuation =
                Continuation::new(span.last(), 0, false, tail_sig);
            let span_idx = id_index(span.span());
            if spans.len() <= span_idx {
                spans.resize_with(
                    span_idx + 1,
                    SpanContinuations::default,
                );
            }

            let SpanContinuations { current, deltas } =
                &mut spans[span_idx];
            match current
                .binary_search_by_key(&(span.last(), tail_sig), |cnt| {
                    (cnt.color(), cnt.tail_sig())
                }) {
                Ok(_) => false,
                Err(pos) => {
                    current.insert(pos, continuation);
                    deltas.push(continuation);
                    true
                },
            }
        } else {
            let Self::Rich(spans) = self else {
                unreachable!(
                    "rich CPS must use rich continuation tables"
                )
            };

            let tail_nz = parity_lower_bound(tail_nz, tail_parity);
            let continuation = Continuation::new(
                span.last(),
                tail_nz,
                tail_parity,
                tail_sig,
            );
            let span_idx = id_index(span.span());
            if spans.len() <= span_idx {
                spans.resize_with(
                    span_idx + 1,
                    SpanContinuations::default,
                );
            }

            let SpanContinuations { current, deltas } =
                &mut spans[span_idx];
            match current.binary_search_by_key(
                &(span.last(), tail_parity, tail_sig),
                |cnt| (cnt.color(), cnt.tail_parity(), cnt.tail_sig()),
            ) {
                Ok(pos) => {
                    if tail_nz < current[pos].tail_nz() {
                        current[pos].set_tail_nz(tail_nz);
                        deltas.push(continuation);
                        return true;
                    }
                    false
                },
                Err(pos) => {
                    current.insert(pos, continuation);
                    deltas.push(continuation);
                    true
                },
            }
        }
    }

    #[inline]
    fn table<const G: u8>(&self) -> &Vec<SpanContinuations> {
        if G == CPS_GOAL_HALT {
            let Self::Halt(spans) = self else {
                unreachable!(
                    "halt CPS must use halt continuation tables"
                )
            };
            spans
        } else {
            let Self::Rich(spans) = self else {
                unreachable!(
                    "rich CPS must use rich continuation tables"
                )
            };
            spans
        }
    }

    #[inline]
    fn current_continuations<const G: u8>(
        &self,
        span: &Span,
    ) -> &Continuations {
        let continuations =
            &self.table::<G>()[id_index(span.span())].current;
        debug_assert!(!continuations.is_empty());
        continuations
    }

    #[inline]
    fn delta_len<const G: u8>(
        &self,
        span: &Span,
    ) -> ContinuationCursor {
        compact_id(
            self.table::<G>()[id_index(span.span())].deltas.len(),
        )
    }

    #[inline]
    fn continuation_deltas<const G: u8>(
        &self,
        span: &Span,
        cursor: ContinuationCursor,
    ) -> &[Continuation] {
        let deltas = &self.table::<G>()[id_index(span.span())].deltas;
        let cursor = id_index(cursor);
        debug_assert!(cursor <= deltas.len());
        &deltas[cursor..]
    }
}

/**************************************/

type Config = config::Config<Tape>;

impl Config {
    fn init(rad: Radius, pool: &mut SpanPool) -> Self {
        Self {
            state: 0,
            tape: Tape::init(rad, pool),
        }
    }
}

/**************************************/

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Tape {
    lspan: Span,
    rspan: Span,
    left_nz: NonzeroCount,
    right_nz: NonzeroCount,
    tail_meta: u16,
    scan: Color,
}

#[expect(clippy::cast_possible_truncation)]
impl Tape {
    const SIG_BITS: u32 = TailSig::USED_BITS as u32;
    const SIG_MASK: u16 = TailSig::PACKED_MASK as u16;
    const RIGHT_SIG_SHIFT: u32 = Self::SIG_BITS;
    const LEFT_PARITY_SHIFT: u32 = Self::SIG_BITS * 2;
    const RIGHT_PARITY_SHIFT: u32 = Self::LEFT_PARITY_SHIFT + 1;

    const fn pack_tail_meta(
        left_parity: bool,
        right_parity: bool,
        left_sig: TailSig,
        right_sig: TailSig,
    ) -> u16 {
        (left_sig.raw() as u16)
            | ((right_sig.raw() as u16) << Self::RIGHT_SIG_SHIFT)
            | ((left_parity as u16) << Self::LEFT_PARITY_SHIFT)
            | ((right_parity as u16) << Self::RIGHT_PARITY_SHIFT)
    }

    const fn left_sig(self) -> TailSig {
        TailSig((self.tail_meta & Self::SIG_MASK) as u8)
    }

    const fn right_sig(self) -> TailSig {
        TailSig(
            ((self.tail_meta >> Self::RIGHT_SIG_SHIFT) & Self::SIG_MASK)
                as u8,
        )
    }

    fn set_left_sig(&mut self, sig: TailSig) {
        self.tail_meta =
            (self.tail_meta & !Self::SIG_MASK) | u16::from(sig.raw());
    }

    fn set_right_sig(&mut self, sig: TailSig) {
        let mask = Self::SIG_MASK << Self::RIGHT_SIG_SHIFT;
        self.tail_meta = (self.tail_meta & !mask)
            | (u16::from(sig.raw()) << Self::RIGHT_SIG_SHIFT);
    }

    const fn left_parity(self) -> bool {
        self.tail_meta & (1 << Self::LEFT_PARITY_SHIFT) != 0
    }

    const fn right_parity(self) -> bool {
        self.tail_meta & (1 << Self::RIGHT_PARITY_SHIFT) != 0
    }

    const fn toggle_left_parity(&mut self) {
        self.tail_meta ^= 1 << Self::LEFT_PARITY_SHIFT;
    }

    const fn toggle_right_parity(&mut self) {
        self.tail_meta ^= 1 << Self::RIGHT_PARITY_SHIFT;
    }

    fn init(rad: Radius, pool: &mut SpanPool) -> Self {
        Self {
            lspan: Span::init(rad, pool),
            rspan: Span::init(rad, pool),
            left_nz: 0,
            right_nz: 0,
            tail_meta: 0,
            scan: 0,
        }
    }

    #[expect(clippy::too_many_arguments)]
    const fn from_spans(
        scan: Color,
        push: Span,
        pull: Span,
        shift: Shift,
        left_nz: NonzeroCount,
        right_nz: NonzeroCount,
        left_parity: bool,
        right_parity: bool,
        left_sig: TailSig,
        right_sig: TailSig,
    ) -> Self {
        let (lspan, rspan) =
            if shift { (push, pull) } else { (pull, push) };

        Self {
            lspan,
            rspan,
            left_nz,
            right_nz,
            tail_meta: Self::pack_tail_meta(
                left_parity,
                right_parity,
                left_sig,
                right_sig,
            ),
            scan,
        }
    }
}

impl config::Scan for Tape {
    fn scan(&self) -> Color {
        self.scan
    }
}

impl fmt::Display for Tape {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "L(pat={}, last={}, nz={}, odd={}, sig={:?}) [{}] R(pat={}, last={}, nz={}, odd={}, sig={:?})",
            self.lspan.span(),
            self.lspan.last(),
            self.left_nz,
            self.left_parity(),
            self.left_sig(),
            self.scan,
            self.rspan.span(),
            self.rspan.last(),
            self.right_nz,
            self.right_parity(),
            self.right_sig()
        )
    }
}

/**************************************/

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Span(u32);

impl Span {
    const ID_MASK: u32 = 0x00ff_ffff;

    const fn raw(self) -> u32 {
        self.0
    }

    const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    fn new(span: SpanId, last: Color) -> Self {
        assert!(
            span <= Self::ID_MASK,
            "CPS span id exceeded 24-bit packing"
        );
        Self(span | (u32::from(last) << 24))
    }

    const fn span(self) -> SpanId {
        self.0 & Self::ID_MASK
    }

    const fn last(self) -> Color {
        (self.0 >> 24) as Color
    }

    fn set_last(&mut self, color: Color) {
        self.0 = (self.0 & Self::ID_MASK) | (u32::from(color) << 24);
    }

    fn init(rad: Radius, pool: &mut SpanPool) -> Self {
        assert!(rad > 0);

        let mut colors = pool.take_colors(rad - 1);
        colors.resize(rad - 1, 0);

        Self::new(pool.intern(colors), 0)
    }

    fn push(&mut self, color: Color, pool: &mut SpanPool) -> Color {
        let span_id = self.span();
        let span_idx = id_index(span_id);
        let last = self.last();

        if let Some(next) = pool.push_cache[span_idx]
            .iter()
            .find(|transition| transition.matches(last, color))
            .map(|transition| transition.next())
        {
            *self = next;
            return last;
        }

        let span_len = pool.colors(span_id).len();
        let mut v = pool.take_colors(span_len);
        let new_last = {
            let colors = pool.colors(span_id);

            if let Some((&last, prefix)) = colors.split_last() {
                v.push(color);
                v.extend_from_slice(prefix);
                last
            } else {
                color
            }
        };

        let next = Self::new(pool.intern(v), new_last);

        pool.push_cache[span_idx]
            .push(PushTransition::new(last, color, next));
        *self = next;
        last
    }

    fn pull(&mut self, pool: &mut SpanPool) -> Color {
        let span_id = self.span();
        let span_idx = id_index(span_id);
        let last = self.last();

        if let Some((next, pulled)) = pool.pull_cache[span_idx]
            .iter()
            .find(|transition| transition.last() == last)
            .map(|transition| (transition.next(), transition.pulled()))
        {
            *self = next;
            return pulled;
        }

        let span_len = pool.colors(span_id).len();
        let mut v = pool.take_colors(span_len);
        let pulled = {
            let colors = pool.colors(span_id);

            if let Some((&first, rest)) = colors.split_first() {
                v.extend_from_slice(rest);
                v.push(last);
                first
            } else {
                last
            }
        };

        let next = Self::new(pool.intern(v), last);

        pool.pull_cache[span_idx]
            .push(PullTransition::new(last, next, pulled));
        *self = next;
        pulled
    }

    fn blank_span(&self, pool: &SpanPool) -> bool {
        pool.blank_span(self.span())
    }

    fn base_blank_span(
        &self,
        prog: &impl GetInstr,
        pool: &mut SpanPool,
    ) -> bool {
        pool.base_blank_span(prog, self.span())
    }

    fn base_all_blank(
        &self,
        prog: &impl GetInstr,
        pool: &mut SpanPool,
    ) -> bool {
        prog.is_blank(self.last()) && self.base_blank_span(prog, pool)
    }
}

#[test]
fn test_tail_sig() {
    fn check<const LEVEL: usize>() {
        let zero = TailSig::default();
        let tail = zero.prepend::<LEVEL>(2);
        let whole = tail.prepend::<LEVEL>(1);

        assert!(whole.is_prepend_of::<LEVEL>(1, tail));
        assert!(!whole.is_prepend_of::<LEVEL>(0, tail));
        assert_eq!(zero.prepend::<LEVEL>(0), zero);
    }

    check::<1>();
    check::<2>();
}

#[test]
fn test_span() {
    let mut pool = SpanPool::new(Halt);
    let mut span = Span::init(3, &mut pool);

    assert_eq!(pool.colors(span.span()).as_slice(), &[0, 0]);
    assert_eq!(span.last(), 0);

    span.push(1, &mut pool);
    span.push(1, &mut pool);
    span.push(0, &mut pool);

    assert_eq!(pool.colors(span.span()).as_slice(), &[0, 1]);
    assert_eq!(span.last(), 1);
}

#[test]
fn test_compact_cps_layouts() {
    use core::mem::size_of;

    assert_eq!(size_of::<TailSig>(), 1);
    assert_eq!(size_of::<Span>(), 4);
    assert_eq!(size_of::<Continuation>(), 4);
    assert_eq!(size_of::<Tape>(), 16);
    assert_eq!(size_of::<ConfigShape>(), 12);
    assert_eq!(size_of::<PushTransition>(), 8);
    assert_eq!(size_of::<PullTransition>(), 8);
    assert_eq!(size_of::<AntichainEntry>(), 8);
    assert_eq!(size_of::<AntichainEntries>(), 16);
    assert_eq!(size_of::<(ConfigId, u32)>(), 8);
}

#[test]
fn test_packed_cps_records_roundtrip() {
    let next = Span::new(123, 9);
    let push = PushTransition::new(4, 6, next);
    assert!(push.matches(4, 6));
    assert!(!push.matches(4, 7));
    assert_eq!(push.next().raw(), next.raw());

    let pull = PullTransition::new(4, next, 8);
    assert_eq!(pull.last(), 4);
    assert_eq!(pull.next().raw(), next.raw());
    assert_eq!(pull.pulled(), 8);
}

#[test]
fn test_inline_antichain_spill() {
    let mut entries = AntichainEntries::empty();
    let mut overflow = Vec::new();
    let mut free_overflow = Vec::new();

    entries.push(
        AntichainEntry::new(1, 4, 10),
        &mut overflow,
        &mut free_overflow,
    );
    entries.push(
        AntichainEntry::new(3, 2, 11),
        &mut overflow,
        &mut free_overflow,
    );
    assert!(entries.overflow_index().is_none());
    assert_eq!(
        entries.find_subsuming(4, 4, &overflow).unwrap().id(),
        10
    );

    entries.push(
        AntichainEntry::new(5, 1, 12),
        &mut overflow,
        &mut free_overflow,
    );
    let index = entries.overflow_index().unwrap();
    assert_eq!(overflow[id_index(index)].len(), 3);
    assert_eq!(
        entries.find_subsuming(5, 1, &overflow).unwrap().id(),
        12
    );
}
