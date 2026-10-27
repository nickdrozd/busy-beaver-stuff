#![expect(clippy::trivially_copy_pass_by_ref)]

use core::{
    cell::Cell,
    cmp::{max, min},
};
use std::{borrow::Cow, collections::HashMap as Dict};

use rayon::prelude::*;

use tm::{
    Color, Goal, Instr, Prog, Shift, Slot, State, Steps,
    config::MedConfig as Config, machine::RunResult,
};

pub type PassConfig<'c> = Cow<'c, Config>;

pub type TreeResult<Harv> = Dict<Instr, Harv>;

type Slots = usize;
type Mask = u16;
type Count = u8;

type Params = (usize, usize);

use RunResult::*;

/**************************************/

const SHIFTS: [Shift; 2] = [false, true];

type Instrs = Vec<Instr>;
type InstrTable = Vec<Vec<Instrs>>;

#[expect(clippy::cast_possible_truncation)]
fn make_instr_table<
    const max_states: usize,
    const max_colors: usize,
>() -> (Instrs, InstrTable) {
    let mut table = vec![vec![vec![]; 1 + max_colors]; 1 + max_states];

    for states in 2..=max_states {
        for colors in 2..=max_colors {
            let mut instrs = Vec::with_capacity(colors * 2 * states);

            for color in 0..colors as Color {
                for shift in SHIFTS {
                    for state in 0..states as State {
                        instrs.push((color, shift, state));
                    }
                }
            }

            table[states][colors] = instrs;
        }
    }

    let init_states = min(3, max_states);
    let init_colors = min(3, max_colors);

    let mut init_instrs = table[init_states][init_colors].clone();

    init_instrs.retain(|instr| !matches!(instr, (_, true, 0 | 1)));

    (init_instrs, table)
}

type SpinoutInstrTable = [InstrTable; 2];

#[expect(clippy::cast_possible_truncation)]
fn make_spinout_table<
    const max_states: usize,
    const max_colors: usize,
>() -> (Instrs, SpinoutInstrTable) {
    let (init_instrs, plain) =
        make_instr_table::<max_states, max_colors>();

    let mut spins: InstrTable =
        vec![vec![vec![]; 1 + max_colors]; 1 + max_states];

    for read in 0..max_states {
        for colors in 2..=max_colors {
            let mut instrs = Vec::with_capacity(colors * 2);

            for color in 0..colors as Color {
                for shift in SHIFTS {
                    instrs.push((color, shift, read as State));
                }
            }

            spins[1 + read][colors] = instrs;
        }
    }

    (init_instrs, [plain, spins])
}

/**************************************/

struct AvailStack<T>(Vec<T>);

impl<T: Copy> AvailStack<T> {
    fn new(val: T) -> Self {
        Self(vec![val])
    }

    fn top(&self) -> T {
        *self.0.last().unwrap()
    }

    fn push(&mut self, val: T) {
        self.0.push(val);
    }

    fn pop(&mut self) {
        self.0.pop();
    }
}

type AvailParams = AvailStack<Params>;

impl AvailParams {
    fn init(states: usize, colors: usize) -> Self {
        Self::new((min(3, states), min(3, colors)))
    }

    fn avail(&self) -> Params {
        self.top()
    }

    fn on_remove(&mut self) {
        self.pop();
    }

    #[expect(clippy::cast_possible_truncation)]
    fn on_insert<const states: usize, const colors: usize>(
        &mut self,
        (slot_st, slot_co): &Slot,
        (instr_co, _, instr_st): &Instr,
    ) {
        let (mut av_st, mut av_co) = self.top();

        if av_st < states
            && 1 + max(slot_st, instr_st) == av_st as State
        {
            av_st += 1;
        }

        if av_co < colors
            && 1 + max(slot_co, instr_co) == av_co as Color
        {
            av_co += 1;
        }

        self.push((av_st, av_co));
    }
}

/**************************************/

trait AvailInstrs<'h, const states: usize, const colors: usize> {
    type Table: Sync;

    fn new(instr_table: &'h Self::Table) -> Self;

    fn avail_instrs(&self, slot: &Slot) -> &'h [Instr];

    fn allows(&self, _: &Slot, _: &Instr) -> bool {
        true
    }

    fn filter_final(
        &self,
        _: &Prog<states, colors>,
        _: &Slot,
        _: &mut Vec<Instr>,
    ) {
    }

    fn on_insert(&mut self, _: &Slot, _: &Instr) {}
    fn on_remove(&mut self) {}
}

struct BasicInstrs<'h> {
    instr_table: &'h InstrTable,
    avail_params: AvailParams,
}

impl<'h, const states: usize, const colors: usize>
    AvailInstrs<'h, states, colors> for BasicInstrs<'h>
{
    type Table = InstrTable;

    fn new(instr_table: &'h Self::Table) -> Self {
        let avail_params = AvailStack::init(states, colors);

        Self {
            instr_table,
            avail_params,
        }
    }

    fn avail_instrs(&self, _: &Slot) -> &'h [Instr] {
        let (st, co) = self.avail_params.avail();

        &self.instr_table[st][co]
    }

    fn on_remove(&mut self) {
        self.avail_params.on_remove();
    }

    fn on_insert(&mut self, slot: &Slot, instr: &Instr) {
        self.avail_params.on_insert::<states, colors>(slot, instr);
    }
}

#[derive(Clone, Copy)]
struct BlankUndo {
    erase_remaining: Option<Count>,
    read: Color,
    remaining: Count,
    prints: Mask,
    can_erase: Mask,
    last_base: Option<(Color, Mask)>,
}

struct AvailBlanks<const colors: usize> {
    // Existing pruning: if no direct erase instruction has been seen yet,
    // this is the number of nonzero-read slots that could still supply one.
    erase_remaining: Option<Count>,

    // Number of undefined slots for each nonzero read color.
    remaining: [Count; colors],

    // prints[c] is a bitset of colors printed by already-defined
    // instructions whose read color is c.
    prints: [Mask; colors],

    // Conservative color -> ... -> 0 reachability. An undefined read-color
    // slot counts as a possible future direct erase.
    can_erase: Mask,

    // DFS rollback: only one read color changes per inserted instruction, so
    // don't copy the whole graph state at every tree edge.
    history: Vec<BlankUndo>,

    // When assigning the final undefined slot for one read color, all
    // candidates share the same closure with that last wildcard removed.
    last_base: Cell<Option<(Color, Mask)>>,
}

impl<const colors: usize> AvailBlanks<colors> {
    fn init(states: usize) -> Self {
        assert!(states <= 16 && colors <= 16);

        let state_count = Count::try_from(states).unwrap();
        let mut remaining = [state_count; colors];
        remaining[0] = 0;

        Self {
            erase_remaining: Some(
                Count::try_from(states * (colors - 1)).unwrap(),
            ),
            remaining,
            prints: [0; colors],
            can_erase: Self::low_bits(colors),
            history: Vec::with_capacity(states * colors),
            last_base: Cell::new(None),
        }
    }

    const fn bit(color: usize) -> Mask {
        1_u16 << color
    }

    const fn low_bits(n: usize) -> Mask {
        if n == 16 { Mask::MAX } else { (1_u16 << n) - 1 }
    }

    // Last three steps I, J, K: J prints zero, K erases a nonzero.
    // If I and J move alike, I must print zero on a cell left behind.
    // If they reverse, K revisits I's cell and must read I's nonzero print.
    // Normalized A0=1RB rules out blanking in fewer than three steps.
    // Build masks once per final batch; no graph updates on tree edges.
    fn filter_final<const states: usize>(
        prog: &Prog<states, colors>,
        &(source, read): &Slot,
        avail_instrs: &mut Vec<Instr>,
    ) {
        let mut ingress = [[0_u16; 2]; states];
        let mut erasers = [0_u16; states];
        for ((state, color), &(print, shift, target)) in prog.iter() {
            ingress[usize::from(target)][usize::from(shift)] |=
                Self::bit(usize::from(print));
            if print == 0 && color != 0 {
                erasers[usize::from(state)] |=
                    Self::bit(usize::from(color));
            }
        }

        // Candidate as I: permitted targets, indexed by its print/direction.
        let mut first = [[0_u16; 2]; colors];
        // Candidate as both I and K, with an existing middle instruction.
        let mut repeated = [0_u16; 2];
        let mut last = false;
        for ((middle, _), &(print, shift, target)) in prog.iter() {
            if print != 0 {
                continue;
            }
            let direction = usize::from(shift);
            let middle_index = usize::from(middle);
            let middle_bit = Self::bit(middle_index);
            let erase = erasers[usize::from(target)];
            let same_zero = ingress[middle_index][direction] & 1 != 0;
            let opposite = ingress[middle_index][direction ^ 1];
            if erase != 0 && (same_zero || opposite & erase != 0) {
                // All three instructions already exist: every candidate survives.
                return;
            }
            if erase != 0 {
                first[0][direction] |= middle_bit;
                let mut pending = erase;
                while pending != 0 {
                    let color = pending.trailing_zeros() as usize;
                    pending &= pending - 1;
                    first[color][direction ^ 1] |= middle_bit;
                }
            }
            if read != 0 && target == source {
                last |= same_zero
                    || opposite & Self::bit(usize::from(read)) != 0;
                repeated[direction] |= middle_bit;
            }
        }

        avail_instrs.retain(|&(print, shift, target)| {
            let direction = usize::from(shift);
            let target_bit = Self::bit(usize::from(target));
            if first[usize::from(print)][direction] & target_bit != 0 {
                return true;
            }
            if print != 0 {
                return false;
            }
            if last || repeated[direction] & target_bit != 0 {
                return true;
            }
            // Candidate as J; it may also supply I or K on another cell.
            let same_state = target == source;
            let erase = erasers[usize::from(target)]
                | if same_state && read != 0 {
                    Self::bit(usize::from(read))
                } else {
                    0
                };
            erase != 0
                && (same_state
                    || ingress[usize::from(source)][direction] & 1 != 0
                    || ingress[usize::from(source)][direction ^ 1]
                        & erase
                        != 0)
        });
    }

    fn must_erase(&self, read: Color) -> bool {
        read != 0 && self.erase_remaining == Some(1)
    }

    fn close(&self, mut can: Mask) -> Mask {
        loop {
            let old = can;

            for color in 1..colors {
                if self.prints[color] & can != 0 {
                    can |= Self::bit(color);
                }
            }

            if can == old {
                return can;
            }
        }
    }

    fn base_without_last(&self, read: Color) -> Mask {
        if let Some((cached_read, can)) = self.last_base.get()
            && cached_read == read
        {
            return can;
        }

        let read_index = usize::from(read);
        debug_assert_eq!(self.remaining[read_index], 1);

        let mut can = Self::bit(0);

        for color in 1..colors {
            if color != read_index && self.remaining[color] != 0 {
                can |= Self::bit(color);
            }
        }

        let can = self.close(can);
        self.last_base.set(Some((read, can)));
        can
    }

    fn allows(
        &self,
        &(_, read): &Slot,
        &(print, _, _): &Instr,
    ) -> bool {
        // With one nonblank color the graph cannot prove anything beyond the
        // existing direct-erase obligation, so remove all graph hot-path work.
        if colors <= 2 || print == 0 {
            return true;
        }

        let read_index = usize::from(read);
        let can = if read != 0 && self.remaining[read_index] == 1 {
            // This candidate consumes the final undefined transition reading
            // `read`. Remove that wildcard once for the entire candidate set.
            self.base_without_last(read)
        } else {
            self.can_erase
        };

        // The candidate immediately writes `print` onto the tape. If that
        // color has no possible rewrite path to 0, this branch can never blank.
        can & Self::bit(usize::from(print)) != 0
    }

    fn on_insert(&mut self, &(_, read): &Slot, &(print, _, _): &Instr) {
        let read_index = usize::from(read);
        let last_base = (colors > 2
            && read != 0
            && self.remaining[read_index] == 1)
            .then(|| self.base_without_last(read));

        self.history.push(BlankUndo {
            erase_remaining: self.erase_remaining,
            read,
            remaining: self.remaining[read_index],
            prints: self.prints[read_index],
            can_erase: self.can_erase,
            last_base: self.last_base.get(),
        });

        self.erase_remaining = if print == 0 && read != 0 {
            None
        } else {
            self.erase_remaining
                .map(|rem| if read != 0 { rem - 1 } else { rem })
        };

        // Binary-color blank enumeration only needs the old direct-erase
        // counter. Avoid maintaining the color graph entirely.
        if colors > 2 && read != 0 {
            let print = usize::from(print);

            debug_assert!(0 < self.remaining[read_index]);
            self.remaining[read_index] -= 1;
            self.prints[read_index] |= Self::bit(print);

            if let Some(base) = last_base {
                // `allows` guarantees that this executed candidate writes an
                // erasable color. The now-complete read color therefore has a
                // rewrite path to 0; close backwards to update predecessors.
                debug_assert_eq!(self.remaining[read_index], 0);
                debug_assert!(
                    print == 0 || base & Self::bit(print) != 0
                );

                self.can_erase =
                    self.close(base | Self::bit(read_index));
            }
        }

        self.last_base.set(None);
    }

    fn on_remove(&mut self) {
        let undo = self.history.pop().unwrap();
        let read = usize::from(undo.read);

        self.erase_remaining = undo.erase_remaining;
        self.remaining[read] = undo.remaining;
        self.prints[read] = undo.prints;
        self.can_erase = undo.can_erase;
        self.last_base.set(undo.last_base);
    }
}

struct BlankInstrs<'h, const colors: usize> {
    instr_table: &'h InstrTable,
    avail_params: AvailParams,
    avail_blanks: AvailBlanks<colors>,
}

impl<'h, const states: usize, const colors: usize>
    AvailInstrs<'h, states, colors> for BlankInstrs<'h, colors>
{
    type Table = InstrTable;

    fn new(instr_table: &'h Self::Table) -> Self {
        let avail_params = AvailStack::init(states, colors);

        let avail_blanks = AvailBlanks::<colors>::init(states);

        Self {
            instr_table,
            avail_params,
            avail_blanks,
        }
    }

    fn avail_instrs(&self, &(_, pr): &Slot) -> &'h [Instr] {
        let (st, co) = self.avail_params.avail();

        let all = &self.instr_table[st][co];

        if self.avail_blanks.must_erase(pr) {
            &all[..2 * st]
        } else {
            all
        }
    }

    fn allows(&self, slot: &Slot, instr: &Instr) -> bool {
        self.avail_blanks.allows(slot, instr)
    }

    fn filter_final(
        &self,
        prog: &Prog<states, colors>,
        slot: &Slot,
        instrs: &mut Vec<Instr>,
    ) {
        AvailBlanks::<colors>::filter_final(prog, slot, instrs);
    }

    fn on_insert(&mut self, slot: &Slot, instr: &Instr) {
        self.avail_params.on_insert::<states, colors>(slot, instr);

        self.avail_blanks.on_insert(slot, instr);
    }

    fn on_remove(&mut self) {
        self.avail_blanks.on_remove();

        self.avail_params.on_remove();
    }
}

#[derive(Clone, Copy)]
struct SpinUndo {
    read_state: State,
    read_color: Color,
    spin: Option<Shift>,
    ingress_r: bool,
    ingress_l: bool,
    target: State,
}

struct AvailSpinouts<const states: usize> {
    // All slots except normalized A0 start undefined. Once the spinout
    // obligation is settled, descendants stop touching the masks/counter and
    // only increment settled_depth until rollback reaches the settling edge.
    undefined: Count,
    settled_depth: Count,

    // State bit is set while that state's blank-read slot is undefined.
    blank_undefined: Mask,

    // Defined blank self-transitions, split by spin direction.
    spin_r: Mask,
    spin_l: Mask,

    // States having a distinct defined ingress compatible with a right/left
    // spin. Same-direction ingress is always compatible; opposite-direction
    // ingress is compatible when it prints 0 and therefore leaves the future
    // spin lane blank.
    ingress_r: Mask,
    ingress_l: Mask,

    history: Vec<SpinUndo>,
}

impl<const states: usize> AvailSpinouts<states> {
    fn init(colors: usize) -> Self {
        assert!(states <= 16 && colors <= 16);

        let all_states = Self::low_bits(states);

        // A0 is normalized to 1RB: it is already defined, so only states
        // B.. remain possible blank self-transition slots. It is also a
        // right-compatible ingress into B.
        let blank_undefined = all_states & !1;
        let ingress_r = if states > 1 { 1_u16 << 1 } else { 0 };

        Self {
            undefined: Count::try_from(states * colors - 1).unwrap(),
            settled_depth: 0,
            blank_undefined,
            spin_r: 0,
            spin_l: 0,
            ingress_r,
            ingress_l: 0,
            history: Vec::with_capacity(states * colors),
        }
    }

    const fn bit(state: State) -> Mask {
        1_u16 << state
    }

    const fn low_bits(n: usize) -> Mask {
        if n == 16 { Mask::MAX } else { (1_u16 << n) - 1 }
    }

    const fn has_spin(&self) -> bool {
        self.spin_r | self.spin_l != 0
    }

    const fn settled(&self) -> bool {
        self.spin_r & self.ingress_r != 0
            || self.spin_l & self.ingress_l != 0
    }

    const fn must_spin(&self, read_color: Color) -> bool {
        read_color == 0
            && !self.has_spin()
            && self.blank_undefined.is_power_of_two()
    }

    const fn viable(
        undefined: Count,
        blank_undefined: Mask,
        spin_r: Mask,
        spin_l: Mask,
        ingress_r: Mask,
        ingress_l: Mask,
    ) -> bool {
        // A complete witness already exists.
        if spin_r & ingress_r != 0 || spin_l & ingress_l != 0 {
            return true;
        }

        // A spin transition exists but lacks ingress. Any still-undefined
        // *other* slot can become a compatible ingress.
        if undefined != 0 && spin_r | spin_l != 0 {
            return true;
        }

        if blank_undefined == 0 {
            return false;
        }

        // A future blank self-transition can use an ingress that already
        // exists; choose its direction to match that ingress.
        if blank_undefined & (ingress_r | ingress_l) != 0 {
            return true;
        }

        // Otherwise one undefined slot is needed for the future spin
        // transition and a distinct second slot for its ingress.
        undefined > 1
    }

    // Rebuild from the table: incremental tracking stops at a one-step
    // witness, which need not satisfy this stronger two-step ingress test.
    // At spin entry the scanned cell and forward ray must all be blank.
    // For predecessors I,J, only two moves along the spin direction allow
    // I to print nonzero. J must print zero if it moves against the spin.
    fn filter_final<const colors: usize>(
        prog: &Prog<states, colors>,
        &(source, read): &Slot,
        instrs: &mut Vec<Instr>,
    ) {
        let mut spins = [0_u16; 2];
        let mut incoming = [0_u16; 2];
        let mut zero_incoming = [0_u16; 2];
        for ((state, color), &(print, shift, target)) in prog.iter() {
            let direction = usize::from(shift);
            let bit = Self::bit(target);
            incoming[direction] |= bit;
            if print == 0 {
                zero_incoming[direction] |= bit;
            }
            if color == 0 && state == target {
                spins[direction] |= bit;
            }
        }
        // A0=1RB can enter a rightward B spin directly, with only one
        // predecessor. Do not require a fictitious step before A0.
        if spins[1] & Self::bit(1) != 0 {
            return;
        }

        // Allowed candidate targets, indexed by [prints zero][direction].
        let mut allowed = [[0_u16; 2]; 2];
        let mut last = [false, read == 0 && source == 1];
        let mut repeated_any = [false; 2];
        let mut repeated_zero = [false; 2];
        let zero_any = zero_incoming[0] | zero_incoming[1];
        for ((middle, color), &(print, shift, target)) in prog.iter() {
            // Choose the last distinct ingress before the uninterrupted spin.
            if color == 0 && middle == target {
                continue;
            }
            let direction = usize::from(shift);
            let middle_bit = Self::bit(middle);
            for spin in 0..2 {
                let same = direction == spin;
                if !same && print != 0 {
                    continue;
                }
                let predecessors = if same {
                    incoming[spin] | zero_incoming[spin ^ 1]
                } else {
                    zero_any
                };
                if spins[spin] & Self::bit(target) != 0 {
                    if predecessors & middle_bit != 0 {
                        // A complete witness already exists, independent of
                        // the candidate; no filtering is necessary.
                        return;
                    }
                    // Candidate supplies I before this existing J and spin.
                    allowed[1][0] |= middle_bit;
                    allowed[1][1] |= middle_bit;
                    if same {
                        allowed[0][spin] |= middle_bit;
                    }
                }
                if read == 0 && target == source {
                    last[spin] |= predecessors & middle_bit != 0;
                    if middle == source {
                        // Candidate supplies both I and the final spin.
                        if same {
                            repeated_any[spin] = true;
                        } else {
                            repeated_zero[spin] = true;
                        }
                    }
                }
            }
        }

        let source_bit = Self::bit(source);
        for direction in 0..2 {
            // Candidate supplies J, optionally also I on a different cell.
            let same =
                incoming[direction] | zero_incoming[direction ^ 1];
            let mut middle_any = if same & source_bit != 0 {
                spins[direction]
            } else {
                0
            };
            let mut middle_zero = middle_any
                | if zero_any & source_bit != 0 {
                    spins[direction ^ 1]
                } else {
                    0
                };
            middle_any |= spins[direction] & source_bit;
            middle_zero |= (spins[0] | spins[1]) & source_bit;
            if read == 0 {
                // A blank self-transition is the spin, not a distinct J.
                middle_any &= !source_bit;
                middle_zero &= !source_bit;
                if last[direction] || repeated_any[direction] {
                    allowed[0][direction] |= source_bit;
                    allowed[1][direction] |= source_bit;
                }
                if repeated_zero[direction] {
                    allowed[1][direction] |= source_bit;
                }
            }
            allowed[0][direction] |= middle_any;
            allowed[1][direction] |= middle_zero;
        }
        instrs.retain(|&(print, shift, target)| {
            allowed[usize::from(print == 0)][usize::from(shift)]
                & Self::bit(target)
                != 0
        });
    }

    fn allows(
        &self,
        &(read_state, read_color): &Slot,
        &(print, shift, target): &Instr,
    ) -> bool {
        debug_assert!(self.undefined != 0 || self.settled());

        // These conditions guarantee that consuming one candidate cannot
        // exhaust the remaining spin+ingress possibilities. They avoid all
        // temporary mask work for almost all upper-tree candidates.
        if self.settled() {
            return true;
        }

        if self.has_spin() && self.undefined > 1 {
            return true;
        }

        if self.undefined > 2 && self.blank_undefined.count_ones() > 1 {
            return true;
        }

        let mut blank_undefined = self.blank_undefined;
        let mut spin_r = self.spin_r;
        let mut spin_l = self.spin_l;
        let mut ingress_r = self.ingress_r;
        let mut ingress_l = self.ingress_l;

        let spin = read_color == 0 && read_state == target;

        if read_color == 0 {
            blank_undefined &= !Self::bit(read_state);
        }

        if spin {
            if shift {
                spin_r |= Self::bit(read_state);
            } else {
                spin_l |= Self::bit(read_state);
            }
        } else {
            let target = Self::bit(target);

            if shift || print == 0 {
                ingress_r |= target;
            }

            if !shift || print == 0 {
                ingress_l |= target;
            }
        }

        Self::viable(
            self.undefined - 1,
            blank_undefined,
            spin_r,
            spin_l,
            ingress_r,
            ingress_l,
        )
    }

    fn on_insert(
        &mut self,
        &(read_state, read_color): &Slot,
        &(print, shift, target): &Instr,
    ) {
        // Once a complete witness exists it can never be invalidated by a
        // descendant definition. Skip all mask/history work until rollback
        // reaches the edge that first settled the obligation.
        if self.settled() {
            self.settled_depth += 1;
            return;
        }

        debug_assert_eq!(self.settled_depth, 0);
        debug_assert!(self.undefined != 0);

        let spin =
            (read_color == 0 && read_state == target).then_some(shift);
        let target_bit = Self::bit(target);

        let add_r = spin.is_none()
            && (shift || print == 0)
            && self.ingress_r & target_bit == 0;
        let add_l = spin.is_none()
            && (!shift || print == 0)
            && self.ingress_l & target_bit == 0;

        self.history.push(SpinUndo {
            read_state,
            read_color,
            spin,
            ingress_r: add_r,
            ingress_l: add_l,
            target,
        });

        self.undefined -= 1;

        if read_color == 0 {
            self.blank_undefined &= !Self::bit(read_state);
        }

        if let Some(shift) = spin {
            if shift {
                self.spin_r |= Self::bit(read_state);
            } else {
                self.spin_l |= Self::bit(read_state);
            }
        } else {
            if add_r {
                self.ingress_r |= target_bit;
            }

            if add_l {
                self.ingress_l |= target_bit;
            }
        }
    }

    fn on_remove(&mut self) {
        if self.settled_depth != 0 {
            self.settled_depth -= 1;
            return;
        }

        let undo = self.history.pop().unwrap();

        self.undefined += 1;

        if undo.read_color == 0 {
            self.blank_undefined |= Self::bit(undo.read_state);
        }

        if let Some(shift) = undo.spin {
            if shift {
                self.spin_r &= !Self::bit(undo.read_state);
            } else {
                self.spin_l &= !Self::bit(undo.read_state);
            }
        }

        let target = Self::bit(undo.target);

        if undo.ingress_r {
            self.ingress_r &= !target;
        }

        if undo.ingress_l {
            self.ingress_l &= !target;
        }
    }
}

struct SpinoutInstrs<'h, const states: usize> {
    instr_table: &'h SpinoutInstrTable,
    avail_params: AvailParams,
    avail_spinouts: AvailSpinouts<states>,
}

impl<'h, const states: usize, const colors: usize>
    AvailInstrs<'h, states, colors> for SpinoutInstrs<'h, states>
{
    type Table = SpinoutInstrTable;

    fn new(instr_table: &'h Self::Table) -> Self {
        let avail_params = AvailStack::init(states, colors);

        let avail_spinouts = AvailSpinouts::<states>::init(colors);

        Self {
            instr_table,
            avail_params,
            avail_spinouts,
        }
    }

    fn avail_instrs(
        &self,
        &(read_state, read_color): &Slot,
    ) -> &'h [Instr] {
        let (st, co) = self.avail_params.avail();

        &(if self.avail_spinouts.must_spin(read_color) {
            &self.instr_table[1][1 + read_state as usize]
        } else {
            &self.instr_table[0][st]
        })[co]
    }

    fn allows(&self, slot: &Slot, instr: &Instr) -> bool {
        self.avail_spinouts.allows(slot, instr)
    }

    fn filter_final(
        &self,
        prog: &Prog<states, colors>,
        slot: &Slot,
        instrs: &mut Vec<Instr>,
    ) {
        AvailSpinouts::<states>::filter_final(prog, slot, instrs);
    }

    fn on_insert(&mut self, slot: &Slot, instr: &Instr) {
        self.avail_params.on_insert::<states, colors>(slot, instr);

        self.avail_spinouts.on_insert(slot, instr);
    }

    fn on_remove(&mut self) {
        self.avail_spinouts.on_remove();

        self.avail_params.on_remove();
    }
}

struct BasicInstrsSmall<'h, const STATES: usize, const COLORS: usize> {
    instrs: &'h [Instr],
}

impl<'h, const states: usize, const colors: usize>
    AvailInstrs<'h, states, colors>
    for BasicInstrsSmall<'h, states, colors>
{
    type Table = InstrTable;

    fn new(instr_table: &'h Self::Table) -> Self {
        Self {
            instrs: &instr_table[states][colors],
        }
    }

    fn avail_instrs(&self, _slot: &Slot) -> &'h [Instr] {
        self.instrs
    }
}

struct BlankInstrsSmall<'h, const states: usize, const colors: usize> {
    instrs_all: &'h [Instr],
    instrs_erase: &'h [Instr],
    avail_blanks: AvailBlanks<colors>,
}

impl<'h, const states: usize, const colors: usize>
    AvailInstrs<'h, states, colors>
    for BlankInstrsSmall<'h, states, colors>
{
    type Table = InstrTable;

    fn new(instr_table: &'h Self::Table) -> Self {
        let instrs_all = &instr_table[states][colors];
        let instrs_erase = &instrs_all[..2 * states];

        let avail_blanks = AvailBlanks::<colors>::init(states);

        Self {
            instrs_all,
            instrs_erase,
            avail_blanks,
        }
    }

    fn avail_instrs(&self, &(_, pr): &Slot) -> &'h [Instr] {
        if self.avail_blanks.must_erase(pr) {
            self.instrs_erase
        } else {
            self.instrs_all
        }
    }

    fn allows(&self, slot: &Slot, instr: &Instr) -> bool {
        self.avail_blanks.allows(slot, instr)
    }

    fn filter_final(
        &self,
        prog: &Prog<states, colors>,
        slot: &Slot,
        instrs: &mut Vec<Instr>,
    ) {
        AvailBlanks::<colors>::filter_final(prog, slot, instrs);
    }

    fn on_insert(&mut self, slot: &Slot, instr: &Instr) {
        self.avail_blanks.on_insert(slot, instr);
    }

    fn on_remove(&mut self) {
        self.avail_blanks.on_remove();
    }
}

/**************************************/

#[derive(Clone, Copy)]
struct HaltReachUndo<const states: usize> {
    predecessors: Mask,
    writers: Mask,
    ancestors: [Mask; states],
}

// Defined instructions only: prove that execution from blank cannot reach
// its first open slot. This subsumes the direction-only reverse-engineer
// test by requiring a state path between a writer and a returning entry.
struct HaltReachability<const states: usize, const colors: usize> {
    // Sources entering a state, indexed by the entry's direction.
    predecessors: [[Mask; 2]; states],
    // Destinations after printing a color, indexed by the writer's direction.
    writers: [[Mask; 2]; colors],
    // Reflexive transitive predecessors: bit u in ancestors[v] means u ->* v.
    ancestors: [Mask; states],
    open: [Mask; states],
    remaining: usize,
}

impl<const states: usize, const colors: usize>
    HaltReachability<states, colors>
{
    const fn init() -> Self {
        assert!(states >= 2 && states <= 16);
        assert!(colors >= 2 && colors <= 16);
        let all = if colors == 16 {
            Mask::MAX
        } else {
            (1_u16 << colors) - 1
        };
        let mut open = [all; states];
        let mut predecessors = [[0; 2]; states];
        let mut writers = [[0; 2]; colors];
        let mut ancestors = [0; states];
        let mut state = 0;
        while state < states {
            ancestors[state] = 1_u16 << state;
            state += 1;
        }
        // Normalized A0 = 1RB is already defined.
        open[0] &= !1;
        predecessors[1][1] = 1;
        writers[1][1] = 1 << 1;
        ancestors[1] |= 1;
        Self {
            predecessors,
            writers,
            ancestors,
            open,
            remaining: states * colors - 1,
        }
    }

    fn insert(
        &mut self,
        slot: &Slot,
        instr: &Instr,
    ) -> HaltReachUndo<states> {
        let source = usize::from(slot.0);
        let print = usize::from(instr.0);
        let target = usize::from(instr.2);
        let direction = usize::from(instr.1);
        let previous = HaltReachUndo {
            predecessors: self.predecessors[target][direction],
            writers: self.writers[print][direction],
            // At most 32 bytes: a fixed-size snapshot avoids allocating an
            // undo log and handles cycles and duplicate edges on rollback.
            ancestors: self.ancestors,
        };
        let source_bit = 1_u16 << source;
        let target_bit = 1_u16 << target;
        debug_assert!(self.open[source] & (1_u16 << slot.1) != 0);
        self.predecessors[target][direction] |= source_bit;
        self.writers[print][direction] |= target_bit;
        self.open[source] &= !(1_u16 << slot.1);
        self.remaining -= 1;

        // Adding u -> v connects every old ancestor of u to every old
        // descendant of v. Capture the source row before updating any rows.
        if self.ancestors[target] & source_bit == 0 {
            let added = self.ancestors[source];
            for ancestors in &mut self.ancestors {
                if *ancestors & target_bit != 0 {
                    *ancestors |= added;
                }
            }
        }
        previous
    }

    fn remove(
        &mut self,
        old: HaltReachUndo<states>,
        slot: &Slot,
        instr: &Instr,
    ) {
        let direction = usize::from(instr.1);
        self.predecessors[usize::from(instr.2)][direction] =
            old.predecessors;
        self.writers[usize::from(instr.0)][direction] = old.writers;
        self.ancestors = old.ancestors;
        self.open[usize::from(slot.0)] |= 1_u16 << slot.1;
        self.remaining += 1;
    }

    // States that can reach any seed with net displacement 0 modulo 3.
    // Prepending a left edge subtracts 1; a right edge adds 1. Each
    // (state, residue) is visited at most once. All storage is on the stack.
    fn zero_mod3_origins(&self, seeds: Mask) -> Mask {
        let mut seen = [seeds, 0_u16, 0_u16];
        let mut pending = seen;
        // [current residue][edge direction], with left = 0, right = 1.
        const PREPEND: [[usize; 2]; 3] = [[2, 1], [0, 2], [1, 0]];

        while pending[0] | pending[1] | pending[2] != 0 {
            for residue in 0..3 {
                while pending[residue] != 0 {
                    let state =
                        pending[residue].trailing_zeros() as usize;
                    pending[residue] &= pending[residue] - 1;
                    for direction in 0..2 {
                        let next = PREPEND[residue][direction];
                        let added = self.predecessors[state][direction]
                            & !seen[next];
                        seen[next] |= added;
                        pending[next] |= added;
                    }
                }
            }
        }
        seen[0]
    }

    // Possible read colors after two defined steps entering `target`.
    // Equal directions visit a new cell: its color is unconstrained.
    // Opposite directions return to the first cell: read must equal print.
    fn ingress_colors(
        &self,
        target: usize,
        prog: &Prog<states, colors>,
        prints: &mut Option<[[Mask; 2]; states]>,
    ) -> Mask {
        let entries = self.predecessors[target];
        if entries[0] | entries[1] == 0 {
            return 0;
        }
        for direction in 0..2 {
            let mut middle = entries[direction];
            while middle != 0 {
                let state = middle.trailing_zeros() as usize;
                middle &= middle - 1;
                if self.predecessors[state][direction] != 0 {
                    return Mask::MAX;
                }
            }
        }
        // Only reversing pairs remain. Build this small table lazily, once
        // per candidate, and share it across all possible open-slot targets.
        let prints = prints.get_or_insert_with(|| {
            let mut incoming = [[0_u16; 2]; states];
            for (_, &(print, shift, next)) in prog.iter() {
                incoming[usize::from(next)][usize::from(shift)] |=
                    1_u16 << print;
            }
            incoming
        });
        let mut reads = 0;
        for direction in 0..2 {
            let mut middle = entries[direction];
            while middle != 0 {
                let state = middle.trailing_zeros() as usize;
                middle &= middle - 1;
                reads |= prints[state][direction ^ 1];
            }
        }
        reads
    }

    fn cannot_halt(&self, prog: &Prog<states, colors>) -> bool {
        if self.remaining == 0 || self.open[0] & 1 != 0 {
            return false;
        }

        // A first open slot reached after A0 alone has no two-step prefix.
        if self.open[1] & 1 != 0 {
            return false;
        }
        let mut prints = None;
        let mut reachable_open = self.open;
        // Try blank witnesses first: they can end the entire check without
        // doing ingress work for nonblank-only rows.
        for state in 0..states {
            if self.open[state] & 1 == 0 {
                continue;
            }
            let reachable = self.open[state]
                & self.ingress_colors(state, prog, &mut prints);
            reachable_open[state] = reachable;
            // Blank cells need no writer, but must still have a compatible
            // two-step ingress. A surviving blank witness is inconclusive.
            if reachable & 1 != 0 {
                return false;
            }
        }

        for state in 0..states {
            let mut pending = reachable_open[state];
            if pending == 0 {
                continue;
            }
            if self.open[state] & 1 == 0 {
                pending &=
                    self.ingress_colors(state, prog, &mut prints);
                if pending == 0 {
                    continue;
                }
            }
            // A write in direction d needs a final entry in direction !d.
            // Aggregate its possible earlier states once for the whole row.
            let mut returns = [0_u16; 2];
            for (direction, origins) in returns.iter_mut().enumerate() {
                let mut predecessors =
                    self.predecessors[state][1 - direction];
                while predecessors != 0 {
                    let source = predecessors.trailing_zeros() as usize;
                    predecessors &= predecessors - 1;
                    *origins |= self.ancestors[source];
                }
            }
            if returns[0] | returns[1] == 0 {
                continue;
            }
            // Blank-read entries were handled above. Nonblank cells must
            // have been written before the first undefined instruction.
            // Compute the stronger graph only for directions with an ordinary
            // witness, then share its result across all open colors here.
            let mut zero_origins = [None; 2];
            pending &= !1;
            while pending != 0 {
                let color = pending.trailing_zeros() as usize;
                pending &= pending - 1;
                for direction in 0..2 {
                    let writers = self.writers[color][direction]
                        & returns[direction];
                    if writers == 0 {
                        continue;
                    }
                    let seeds = self.predecessors[state][1 - direction];
                    // Empty paths already have displacement zero. Avoid
                    // building the residue graph for this common witness.
                    if writers & seeds != 0 {
                        return false;
                    }
                    let origins = zero_origins[direction]
                        .unwrap_or_else(|| {
                            let origins = self.zero_mod3_origins(seeds);
                            zero_origins[direction] = Some(origins);
                            origins
                        });
                    if writers & origins != 0 {
                        return false;
                    }
                }
            }
        }
        true
    }
}

#[derive(Clone, Copy)]
struct TreeUndo {
    used_states: Count,
    used_colors: Count,
    defined_reads: Mask,
    targets: Mask,
    complete: Mask,
    terminal: Mask,
}

struct Tree<const states: usize, const colors: usize, AvIn, Harv> {
    prog: Prog<states, colors>,
    halt_reachability: HaltReachability<states, colors>,
    instrs: AvIn,
    sim_lim: Steps,
    remaining_slots: Slots,
    defined: Count,
    used_states: Count,
    used_colors: Count,
    defined_reads: [Mask; states],
    targets: [Mask; states],
    complete: Mask,
    terminal: Mask,
    // Cached outward blank-ray results, keyed by direction, whether the
    // undefined slot reads blank, and its state (unused for nonblank reads).
    ray_cache: [[[Option<Mask>; states]; 2]; 2],
    harvester: Harv,
}

impl<
    'i,
    const states: usize,
    const colors: usize,
    AvIn: AvailInstrs<'i, states, colors>,
    Harv: Harvester<states, colors>,
> Tree<states, colors, AvIn, Harv>
{
    const fn init(
        halt: Slots,
        sim_lim: Steps,
        harvester: Harv,
        instrs: AvIn,
    ) -> Self {
        let prog = Prog::<states, colors>::init_norm();

        let remaining_slots = (states * colors) - halt - 2;

        let mut defined_reads = [0; states];
        defined_reads[0] = 1;

        let mut targets = [0; states];
        targets[0] = 1_u16 << 1;

        Self {
            prog,
            halt_reachability: HaltReachability::init(),
            instrs,
            sim_lim,
            remaining_slots,
            // Normalized A0 = 1RB is the one initial definition and already
            // introduces states A/B and colors 0/1.
            defined: 1,
            used_states: 2,
            used_colors: 2,
            defined_reads,
            targets,
            complete: 0,
            terminal: 0,
            ray_cache: [[[None; states]; 2]; 2],
            harvester,
        }
    }

    const fn final_slot(&self) -> bool {
        self.remaining_slots == 0
    }

    #[expect(clippy::cast_possible_truncation)]
    fn shape_after(
        &self,
        slot: &Slot,
        instr: &Instr,
    ) -> (Count, Count) {
        let used_states = max(
            usize::from(self.used_states),
            1 + usize::from(max(slot.0, instr.2)),
        );
        let used_colors = max(
            usize::from(self.used_colors),
            1 + usize::from(max(slot.1, instr.0)),
        );

        (used_states as Count, used_colors as Count)
    }

    const fn low_bits(n: usize) -> Mask {
        if n == 16 { Mask::MAX } else { (1_u16 << n) - 1 }
    }

    fn compute_complete_states(&self) -> Mask {
        let used_reads = Self::low_bits(usize::from(self.used_colors));
        let mut complete = 0;

        for state in 0..usize::from(self.used_states) {
            if self.defined_reads[state] == used_reads {
                complete |= 1_u16 << state;
            }
        }

        complete
    }

    const fn terminal_states(
        &self,
        mut complete: Mask,
        source_override: Option<(usize, Mask)>,
    ) -> Mask {
        loop {
            let old = complete;
            let mut pending = complete;

            while pending != 0 {
                let state = pending.trailing_zeros() as usize;
                let bit = 1_u16 << state;
                pending &= !bit;

                let targets = match source_override {
                    Some((source, targets)) if source == state => {
                        targets
                    },
                    _ => self.targets[state],
                };

                if targets & !complete != 0 {
                    complete &= !bit;
                }
            }

            if complete == old {
                return complete;
            }
        }
    }

    // Find states closed over `reads`, which includes every possible tape
    // color after one candidate step, including its print color and blank.
    // Treat the current slot as defined but omit its target edge. Any target
    // inside the resulting set keeps that set closed when the edge is added.
    // The print is already in `reads`, so this proof is shared by all
    // candidate targets, directions, and prints with the same color mask.
    #[expect(clippy::cast_possible_truncation)]
    fn restricted_terminal_states(
        &self,
        slot: &Slot,
        reads: Mask,
    ) -> Mask {
        let source = usize::from(slot.0);
        let source_read = 1_u16 << slot.1;
        let mut closed = 0_u16;
        let mut targets = [0_u16; states];

        for state in 0..usize::from(self.used_states) {
            let defined = self.defined_reads[state]
                | if state == source { source_read } else { 0 };
            if reads & !defined != 0 {
                continue;
            }

            let mut pending = reads;
            if state == source {
                pending &= !source_read;
            }
            let mut stays = true;
            while pending != 0 {
                let color = pending.trailing_zeros() as Color;
                pending &= pending - 1;
                let &(print, _, target) =
                    self.prog.get(&(state as State, color)).unwrap();
                if reads & (1_u16 << print) == 0 {
                    stays = false;
                    break;
                }
                targets[state] |= 1_u16 << target;
            }
            if stays {
                closed |= 1_u16 << state;
            }
        }

        loop {
            let old = closed;
            let mut pending = closed;
            while pending != 0 {
                let state = pending.trailing_zeros() as usize;
                let bit = 1_u16 << state;
                pending &= pending - 1;
                if targets[state] & !closed != 0 {
                    closed &= !bit;
                }
            }
            if closed == old {
                return closed;
            }
        }
    }

    fn blank_ray_targets(
        &mut self,
        source: State,
        source_blank: bool,
        shift: Shift,
    ) -> Mask {
        let direction = usize::from(shift);
        let blank = usize::from(source_blank);
        let source_key =
            if source_blank { usize::from(source) } else { 0 };

        if let Some(dead) = self.ray_cache[direction][blank][source_key]
        {
            return dead;
        }

        let dead =
            self.compute_blank_ray_targets(source, source_blank, shift);
        self.ray_cache[direction][blank][source_key] = Some(dead);
        dead
    }

    fn compute_blank_ray_targets(
        &self,
        source: State,
        source_blank: bool,
        shift: Shift,
    ) -> Mask {
        // An undefined source0 closes an outward ray through the candidate.
        let dead = if source_blank { 1_u16 << source } else { 0 };
        self.extend_blank_ray_targets(dead, shift)
    }

    // Also used to find states that move monotonically outward until they
    // enter a short outward-drifting cycle.
    fn extend_blank_ray_targets(
        &self,
        mut dead: Mask,
        shift: Shift,
    ) -> Mask {
        let mut live = 0;

        for start in 0..usize::from(self.used_states) {
            let start_bit = 1_u16 << start;
            if (dead | live) & start_bit != 0 {
                continue;
            }

            let mut seen = 0;
            #[expect(clippy::cast_possible_truncation)]
            let mut state = start as State;

            let reaches_dead = loop {
                let state_bit = 1_u16 << state;

                if dead & state_bit != 0 {
                    break true;
                }
                if live & state_bit != 0 {
                    break false;
                }
                if seen & state_bit != 0 {
                    break true;
                }
                seen |= state_bit;

                let Some(&(_, next_shift, next_state)) =
                    self.prog.get(&(state, 0))
                else {
                    break false;
                };

                if next_shift != shift {
                    break false;
                }

                state = next_state;
            };

            if reaches_dead {
                dead |= seen;
            } else {
                live |= seen;
            }
        }

        dead
    }

    // On a fresh blank ray, recognize three-step (out/in/out) and
    // four-step (out/in/out/out or out/out/in/out) drifting cycles.
    // All finish in the starting state on a fresh blank cell, without
    // crossing behind the original head position. In the out/out/in/out
    // case, the third step must leave its cell blank because that cell
    // becomes the starting position of the next cycle.
    // `get` may include the candidate as a slot override.
    #[inline]
    fn short_ray(
        start: State,
        (first_print, outward, second_state): Instr,
        mut get: impl FnMut(State, Color) -> Option<Instr>,
    ) -> bool {
        let Some((second_print, second_shift, third_state)) =
            get(second_state, 0)
        else {
            return false;
        };

        if second_shift == outward {
            // Out/out/in/out: the third step visits a second fresh blank
            // cell, and must leave it blank for the next cycle to begin.
            let Some((third_print, third_shift, fourth_state)) =
                get(third_state, 0)
            else {
                return false;
            };
            return third_print == 0
                && third_shift != outward
                && matches!(
                    get(fourth_state, second_print),
                    Some((_, fourth_shift, final_state))
                        if fourth_shift == outward && final_state == start
                );
        }

        // Out/in/out: the third step returns to the original cell and
        // therefore reads the color written by the first step.
        let Some((_, third_shift, fourth_state)) =
            get(third_state, first_print)
        else {
            return false;
        };
        if third_shift != outward {
            return false;
        }

        // After three steps the head is one cell outward. It must still be
        // blank, and the machine must be back in its starting state.
        if second_print == 0 && fourth_state == start {
            return true;
        }

        // Otherwise check a fourth outward step, which lands on another
        // fresh blank cell in the starting state.
        matches!(
            get(fourth_state, second_print),
            Some((_, fourth_shift, final_state))
                if fourth_shift == outward && final_state == start
        )
    }

    // Unlike the simple blank-ray cache, this proof depends on nonblank
    // instructions, so calculate it only for the current branch.
    #[expect(clippy::cast_possible_truncation)]
    fn short_ray_targets(&self, blank_sides: [bool; 2]) -> [Mask; 2] {
        let mut cycles = [0; 2];

        for start in 0..usize::from(self.used_states) {
            let start = start as State;

            let Some(&first) = self.prog.get(&(start, 0)) else {
                continue;
            };
            let direction = usize::from(first.1);
            if !blank_sides[direction] {
                continue;
            }

            if Self::short_ray(start, first, |state, color| {
                self.prog.get(&(state, color)).copied()
            }) {
                cycles[direction] |= 1_u16 << start;
            }
        }

        // An all-outward blank-read prefix may lead into either cycle.
        for (direction, starts) in cycles.iter_mut().enumerate() {
            if *starts != 0 {
                *starts = self
                    .extend_blank_ray_targets(*starts, direction != 0);
            }
        }
        cycles
    }

    // The current undefined blank-read instruction may begin the same cycle.
    // Successor lookups include that candidate if the path revisits its slot.
    fn candidate_short_ray(&self, slot: &Slot, instr: &Instr) -> bool {
        if slot.1 != 0 {
            return false;
        }

        Self::short_ray(slot.0, *instr, |state, color| {
            if (state, color) == *slot {
                Some(*instr)
            } else {
                self.prog.get(&(state, color)).copied()
            }
        })
    }

    fn on_shape_insert(
        &mut self,
        slot: &Slot,
        instr: &Instr,
    ) -> TreeUndo {
        if slot.1 == 0 {
            self.ray_cache = [[[None; states]; 2]; 2];
        }

        let state = usize::from(slot.0);
        let previous = TreeUndo {
            used_states: self.used_states,
            used_colors: self.used_colors,
            defined_reads: self.defined_reads[state],
            targets: self.targets[state],
            complete: self.complete,
            terminal: self.terminal,
        };

        (self.used_states, self.used_colors) =
            self.shape_after(slot, instr);
        self.defined += 1;

        self.defined_reads[state] |= 1_u16 << slot.1;
        self.targets[state] |= 1_u16 << instr.2;

        #[expect(clippy::if_not_else)]
        if self.used_colors != previous.used_colors {
            self.complete = self.compute_complete_states();
            self.terminal = self.terminal_states(self.complete, None);
        } else {
            let used_reads =
                Self::low_bits(usize::from(self.used_colors));
            let state_bit = 1_u16 << slot.0;

            if self.defined_reads[state] == used_reads {
                self.complete |= state_bit;
                self.terminal =
                    self.terminal_states(self.complete, None);
            }
        }

        previous
    }

    fn on_shape_remove(&mut self, previous: TreeUndo, slot: &Slot) {
        if slot.1 == 0 {
            self.ray_cache = [[[None; states]; 2]; 2];
        }

        self.defined -= 1;
        self.used_states = previous.used_states;
        self.used_colors = previous.used_colors;

        let state = usize::from(slot.0);
        self.defined_reads[state] = previous.defined_reads;
        self.targets[state] = previous.targets;
        self.complete = previous.complete;
        self.terminal = previous.terminal;
    }

    fn run(&self, config: &mut Config) -> RunResult {
        self.prog.run_basic(self.sim_lim, config)
    }

    fn not_term_soon(&self, config: &mut Config) -> bool {
        matches!(self.run(config), StepLimit)
    }

    fn harvest(&mut self, config: &mut PassConfig<'_>) {
        self.harvester.harvest(&self.prog, config);
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cognitive_complexity
    )]
    fn branch(&mut self, mut config: Config) {
        let slot @ (slot_state, _) = match self.run(&mut config) {
            Undefined(slot) => slot,
            Blank | Spinout => return,
            StepLimit => {
                self.harvest(&mut PassConfig::Owned(config));
                return;
            },
            _ => {
                unreachable!()
            },
        };

        let mut avail_instrs: Vec<_> =
            self.instrs.avail_instrs(&slot).into();

        // If a candidate moves into a blank span, follow the already-defined
        // blank-read transitions in that same direction. The scanned cell
        // itself need not be blank.
        // Reaching a state cycle means the head drifts outward forever and can
        // never encounter another undefined slot. When the current slot is S0,
        // also treat reaching S as closing the cycle through this candidate.
        let source_blank = slot.1 == 0;
        let ray_l = if config.tape.lspan.blank() {
            self.blank_ray_targets(slot_state, source_blank, false)
        } else {
            0
        };
        let ray_r = if config.tape.rspan.blank() {
            self.blank_ray_targets(slot_state, source_blank, true)
        } else {
            0
        };

        if ray_l | ray_r != 0 {
            avail_instrs.retain(|&(_, shift, target)| {
                let dead = if shift { ray_r } else { ray_l };
                dead & (1_u16 << target) == 0
            });
        }

        if config.tape.scan != 0
            && config.tape.lspan.blank()
            && config.tape.rspan.blank()
        {
            avail_instrs.retain(|&(pr, _, _)| pr != 0);
        }

        if !self.final_slot() {
            // Rectangle pruning can only fire when the current slot is the
            // final undefined slot in the currently introduced state×color
            // rectangle. Compute that once for the whole candidate set.
            let must_expand = usize::from(self.defined) + 1
                == usize::from(self.used_states)
                    * usize::from(self.used_colors);

            let used_states = usize::from(self.used_states);
            let used_colors = usize::from(self.used_colors);

            if must_expand {
                avail_instrs.retain(|&(print, _, target)| {
                    usize::from(target) >= used_states
                        || usize::from(print) >= used_colors
                });
            } else {
                // A complete state set is terminal when every transition from
                // every state in the set stays inside the set. Entering such a
                // set cannot reach another undefined slot. If the candidate
                // completes the source row, find the terminal set *before*
                // adding its target edge. Adding an edge into that set cannot
                // change it; adding an edge outside it cannot make that target
                // terminal. Thus one closure handles every candidate target.
                // A newly introduced color opens a slot in every row, so it
                // remains exempt from this pruning.
                let used_reads = Self::low_bits(used_colors);
                let source = usize::from(slot_state);
                let source_bit = 1_u16 << slot_state;
                let closes_row = self.defined_reads[source]
                    | (1_u16 << slot.1)
                    == used_reads;
                let dead_targets = if closes_row {
                    self.terminal_states(
                        self.complete | source_bit,
                        Some((source, self.targets[source])),
                    )
                } else {
                    self.terminal
                };

                if dead_targets != 0 {
                    avail_instrs.retain(|&(print, _, target)| {
                        usize::from(print) >= used_colors
                            || dead_targets & (1_u16 << target) == 0
                    });
                }
            }
        }

        avail_instrs.retain(|instr| self.instrs.allows(&slot, instr));

        // Scan span colors once, excluding the current cell because the
        // candidate overwrites it. Stop as soon as all introduced colors are
        // present: the full-alphabet terminal check above already covers that
        // case. Counts and candidate direction do not affect this mask.
        if colors > 2 && !self.final_slot() && !avail_instrs.is_empty()
        {
            let used_reads =
                Self::low_bits(usize::from(self.used_colors));
            let mut span_colors = 1_u16;
            for block in
                config.tape.lspan.iter().chain(config.tape.rspan.iter())
            {
                span_colors |= 1_u16 << block.color;
                if span_colors == used_reads {
                    break;
                }
            }

            if span_colors != used_reads {
                let mut checked = 0_u16;
                let mut terminal = [0_u16; colors];

                avail_instrs.retain(|&(print, _, target)| {
                    if usize::from(print)
                        >= usize::from(self.used_colors)
                        || usize::from(target)
                            >= usize::from(self.used_states)
                    {
                        return true;
                    }
                    let print_bit = 1_u16 << print;
                    let reads = span_colors | print_bit;
                    if reads == used_reads {
                        return true;
                    }

                    // Every print already present in the spans has the same
                    // mask. Blank is always present, so cache all those under
                    // index 0; other prints each add one distinct color.
                    let key = if span_colors & print_bit != 0 {
                        0
                    } else {
                        usize::from(print)
                    };
                    let key_bit = 1_u16 << key;
                    if checked & key_bit == 0 {
                        checked |= key_bit;
                        terminal[key] = self
                            .restricted_terminal_states(&slot, reads);
                    }
                    terminal[key] & (1_u16 << target) == 0
                });
            }
        }

        // Reject exact two-step stationary loops. For each existing target
        // state, precompute which candidate directions would return to the
        // original state and tape without changing either visited cell.
        // Only a candidate that prints the scanned color can close the loop.
        if !avail_instrs.is_empty() {
            let adjacent = [
                config
                    .tape
                    .lspan
                    .first()
                    .map_or(0, |block| block.color),
                config
                    .tape
                    .rspan
                    .first()
                    .map_or(0, |block| block.color),
            ];
            let same_adjacent = adjacent[0] == adjacent[1];
            let mut dead = [0_u16; 2];
            let source_bit = 1_u16 << slot_state;

            for target in 0..usize::from(self.used_states) {
                // The second step must return to the original state.
                // Ignore states with no defined transition targeting it.
                if self.targets[target] & source_bit == 0 {
                    continue;
                }

                let target_state = target as State;
                let target_bit = 1_u16 << target;

                // First lookup handles candidates moving left. If both
                // adjacent colors match, it handles rightward candidates too.
                if let Some(&(print, back, state)) =
                    self.prog.get(&(target_state, adjacent[0]))
                    && print == adjacent[0]
                    && state == slot_state
                {
                    if back {
                        dead[0] |= target_bit;
                    } else if same_adjacent {
                        dead[1] |= target_bit;
                    }
                }

                // Different adjacent colors require a second lookup.
                if !same_adjacent
                    && matches!(
                        self.prog.get(&(target_state, adjacent[1])),
                        Some(&(print, false, state))
                            if print == adjacent[1] && state == slot_state
                    )
                {
                    dead[1] |= target_bit;
                }
            }

            if dead[0] | dead[1] != 0 {
                avail_instrs.retain(|&(print, shift, target)| {
                    print != slot.1
                        || dead[usize::from(shift)] & (1_u16 << target)
                            == 0
                });
            }
        }

        // A candidate can enter a proven blank ray by crossing exactly one
        // adjacent cell. Remember those sides for the short-ray check below;
        // the intervening defined instruction may also enter a drifting ray.
        // Only use rays proved without the current candidate, since an
        // intervening nonblank cell could invalidate a candidate-based proof.
        let mut single_cells = [None; 2];
        if !avail_instrs.is_empty() {
            let spans = [&config.tape.lspan, &config.tape.rspan];
            let mut passage = [0_u16; 2];

            for (direction, span) in spans.into_iter().enumerate() {
                if span.len() != 1 {
                    continue;
                }
                let block = span.first().unwrap();
                if block.count != 1 {
                    continue;
                }
                single_cells[direction] = Some(block.color);

                let outward = direction != 0;
                let ray =
                    self.blank_ray_targets(slot_state, false, outward);
                if ray == 0 {
                    continue;
                }

                for target in 0..usize::from(self.used_states) {
                    if let Some(&(_, shift, next_state)) =
                        self.prog.get(&(target as State, block.color))
                        && shift == outward
                        && ray & (1_u16 << next_state) != 0
                    {
                        passage[direction] |= 1_u16 << target;
                    }
                }
            }

            if passage[0] | passage[1] != 0 {
                avail_instrs.retain(|&(_, shift, target)| {
                    passage[usize::from(shift)] & (1_u16 << target) == 0
                });
            }
        }

        // Short outward-drifting cycles may reverse over fresh blank cells.
        // Reuse their defined-only target masks for a direct blank entry or a
        // one-cell passage. Compute both directions in a single graph scan.
        if !avail_instrs.is_empty() {
            let blank_sides =
                [config.tape.lspan.blank(), config.tape.rspan.blank()];
            let ray_sides = [
                blank_sides[0] || single_cells[0].is_some(),
                blank_sides[1] || single_cells[1].is_some(),
            ];
            if ray_sides[0] || ray_sides[1] {
                let rays = self.short_ray_targets(ray_sides);
                let mut dead = [0_u16; 2];

                for direction in 0..2 {
                    let ray = rays[direction];
                    if ray == 0 {
                        continue;
                    }

                    // Direct entry into a drifting cycle on fresh blank tape.
                    if blank_sides[direction] {
                        dead[direction] = ray;
                        continue;
                    }

                    // The candidate moves over one cell, then a defined
                    // instruction moves outward into the proven short ray.
                    let Some(color) = single_cells[direction] else {
                        continue;
                    };
                    let outward = direction != 0;

                    for target in 0..usize::from(self.used_states) {
                        // Any valid second transition must target the ray.
                        if self.targets[target] & ray == 0 {
                            continue;
                        }

                        if let Some(&(_, shift, next_state)) =
                            self.prog.get(&(target as State, color))
                            && shift == outward
                            && ray & (1_u16 << next_state) != 0
                        {
                            dead[direction] |= 1_u16 << target;
                        }
                    }
                }

                if dead[0] | dead[1] != 0 {
                    avail_instrs.retain(|&(_, shift, target)| {
                        dead[usize::from(shift)] & (1_u16 << target)
                            == 0
                    });
                }

                // The defined-only mask excludes cycles completed by the
                // candidate. Check those separately, only on blank spans.
                if source_blank
                    && !avail_instrs.is_empty()
                    && (blank_sides[0] || blank_sides[1])
                {
                    avail_instrs.retain(|instr @ &(_, shift, _)| {
                        !blank_sides[usize::from(shift)]
                            || !self.candidate_short_ray(&slot, instr)
                    });
                }
            }
        }

        if self.final_slot() && !avail_instrs.is_empty() {
            self.instrs.filter_final(
                &self.prog,
                &slot,
                &mut avail_instrs,
            );
        }

        let Some((last_instr, instrs)) = avail_instrs.split_last()
        else {
            return;
        };

        if self.final_slot() {
            for next_instr in instrs {
                self.prog.insert(&slot, next_instr);

                if !self.prog.term_immediate(&config, *next_instr) {
                    self.harvest(&mut PassConfig::Borrowed(&config));
                }
            }

            {
                self.prog.insert(&slot, last_instr);

                if self.not_term_soon(&mut config) {
                    self.harvest(&mut PassConfig::Owned(config));
                }
            }

            self.prog.remove(&slot);

            return;
        }

        config.state = slot_state;

        self.remaining_slots -= 1;

        for next_instr in instrs {
            self.prog.insert(&slot, next_instr);
            let reverse =
                self.halt_reachability.insert(&slot, next_instr);

            // Internal candidates only: no reachable open slot means this
            // program cannot grow another branch. Reject before cloning.
            if !self.halt_reachability.cannot_halt(&self.prog) {
                let shape = self.on_shape_insert(&slot, next_instr);
                self.instrs.on_insert(&slot, next_instr);

                self.branch(config.clone());

                self.instrs.on_remove();
                self.on_shape_remove(shape, &slot);
            }
            self.halt_reachability.remove(reverse, &slot, next_instr);
        }

        {
            self.prog.insert(&slot, last_instr);
            let reverse =
                self.halt_reachability.insert(&slot, last_instr);

            // Internal candidates only: no reachable open slot means this
            // program cannot grow another branch. Reject before cloning.
            if !self.halt_reachability.cannot_halt(&self.prog) {
                let shape = self.on_shape_insert(&slot, last_instr);
                self.instrs.on_insert(&slot, last_instr);

                self.branch(config);

                self.instrs.on_remove();
                self.on_shape_remove(shape, &slot);
            }
            self.halt_reachability.remove(reverse, &slot, last_instr);
        }

        self.prog.remove(&slot);

        self.remaining_slots += 1;
    }

    fn run_branch(
        init_instrs: &Instrs,
        halt: Slots,
        sim_lim: Steps,
        instr_table: &'i AvIn::Table,
        harvester: impl Send + Sync + Fn() -> Harv,
    ) -> TreeResult<Harv> {
        init_instrs
            .par_iter()
            .map(|instr| {
                let mut tree = Self::init(
                    halt,
                    sim_lim,
                    harvester(),
                    AvIn::new(instr_table),
                );

                tree.remaining_slots -= 1;
                tree.prog.insert(&INIT_SLOT, instr);
                tree.halt_reachability.insert(&INIT_SLOT, instr);
                tree.on_shape_insert(&INIT_SLOT, instr);
                tree.instrs.on_insert(&INIT_SLOT, instr);

                tree.branch(Config::init_stepped());

                (*instr, tree.harvester)
            })
            .collect()
    }
}

const INIT_SLOT: Slot = (1, 0);

type BasicTree<'i, const s: usize, const c: usize, H> =
    Tree<s, c, BasicInstrs<'i>, H>;

type BasicTreeSmall<'i, const s: usize, const c: usize, H> =
    Tree<s, c, BasicInstrsSmall<'i, s, c>, H>;

type BlankTree<'i, const s: usize, const c: usize, H> =
    Tree<s, c, BlankInstrs<'i, c>, H>;

type BlankTreeSmall<'i, const s: usize, const c: usize, H> =
    Tree<s, c, BlankInstrsSmall<'i, s, c>, H>;

type SpinoutTree<'i, const s: usize, const c: usize, H> =
    Tree<s, c, SpinoutInstrs<'i, s>, H>;

/**************************************/

pub trait Harvester<const states: usize, const colors: usize>:
    Send + Sized
{
    fn harvest(
        &mut self,
        prog: &Prog<states, colors>,
        config: &mut PassConfig<'_>,
    );

    type Output;

    fn combine(results: &TreeResult<Self>) -> Self::Output;

    fn run_params(
        goal: Option<Goal>,
        sim_lim: Steps,
        harvester: &(impl Send + Sync + Fn() -> Self),
    ) -> Self::Output {
        let results = match goal {
            Some(Goal::Halt) | None => Self::run_all(
                Slots::from(goal.is_some()),
                sim_lim,
                harvester,
            ),
            Some(Goal::Blank) => Self::run_blank(sim_lim, harvester),
            Some(Goal::Spinout) => {
                Self::run_spinout(sim_lim, harvester)
            },
        };

        Self::combine(&results)
    }

    fn run_instrs<const instrs: usize>(
        sim_lim: Steps,
        harvester: &(impl Send + Sync + Fn() -> Self),
    ) -> Self::Output {
        assert_eq!(states, instrs);
        assert_eq!(colors, instrs);

        let results = Self::run_all(
            (instrs * instrs) - instrs,
            sim_lim,
            harvester,
        );

        Self::combine(&results)
    }

    fn run_all(
        halt: Slots,
        sim_lim: Steps,
        harvester: &(impl Send + Sync + Fn() -> Self),
    ) -> TreeResult<Self> {
        let (init_instrs, instr_table) =
            make_instr_table::<states, colors>();

        let basic_runner = if states <= 3 && colors <= 3 {
            BasicTreeSmall::run_branch
        } else {
            BasicTree::run_branch
        };

        basic_runner(
            &init_instrs,
            halt,
            sim_lim,
            &instr_table,
            harvester,
        )
    }

    fn run_blank(
        sim_lim: Steps,
        harvester: &(impl Send + Sync + Fn() -> Self),
    ) -> TreeResult<Self> {
        let (init_instrs, instr_table) =
            make_instr_table::<states, colors>();

        let runner = if states <= 3 && colors <= 3 {
            BlankTreeSmall::run_branch
        } else {
            BlankTree::run_branch
        };

        runner(&init_instrs, 0, sim_lim, &instr_table, harvester)
    }

    fn run_spinout(
        sim_lim: Steps,
        harvester: &(impl Send + Sync + Fn() -> Self),
    ) -> TreeResult<Self> {
        let (init_instrs, instr_table) =
            make_spinout_table::<states, colors>();

        // A right-moving B0 self-transition already has a compatible
        // distinct ingress from normalized A0 = 1RB, so those roots need no
        // further spinout-specific structural tracking.
        let (init_done, mut init_need): (Instrs, Instrs) = init_instrs
            .into_iter()
            .partition(|&(_, shift, target)| shift && target == 1);

        let mut result = BasicTree::run_branch(
            &init_done,
            0,
            sim_lim,
            &instr_table[0],
            harvester,
        );

        // With two states B0 is the only remaining blank-read slot, so a
        // non-self-target root can never acquire a spin witness later.
        if states == 2 {
            init_need.retain(|&(_, _, target)| target == 1);
        }

        result.extend(SpinoutTree::run_branch(
            &init_need,
            0,
            sim_lim,
            &instr_table,
            harvester,
        ));

        result
    }
}
