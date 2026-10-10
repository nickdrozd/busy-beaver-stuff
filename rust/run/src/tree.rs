#![expect(clippy::trivially_copy_pass_by_ref)]

use core::{
    cell::Cell,
    cmp::{max, min},
};
use std::{borrow::Cow, collections::HashMap as Dict};

use rayon::prelude::*;

use tm::{
    Color, Goal, Instr, Prog, Shift, Slot, State, Steps,
    config::MedConfig as Config, instrs::Parse as _,
    machine::RunResult,
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

    fn on_insert(&mut self, slot: &Slot, instr: &Instr) {
        self.avail_blanks.on_insert(slot, instr);
    }

    fn on_remove(&mut self) {
        self.avail_blanks.on_remove();
    }
}

/**************************************/

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

    // Overapproximate the states and tape colors reachable after executing a
    // candidate. If every state/color pair in that closure is defined, no
    // future execution can encounter an undefined slot. The caller only uses
    // this when both tape spans are blank, so initially the tape can contain
    // only 0 and the candidate's printed color.
    #[expect(clippy::cast_possible_truncation)]
    fn restricted_terminal(&self, slot: &Slot, instr: &Instr) -> bool {
        let mut reachable_states = 1_u16 << instr.2;
        let mut reachable_colors = 1_u16 | (1_u16 << instr.0);
        let mut processed = [0_u16; states];

        loop {
            let mut pending_states = reachable_states;
            let mut progressed = false;

            while pending_states != 0 {
                let state = pending_states.trailing_zeros() as usize;
                pending_states &= pending_states - 1;

                let mut reads = reachable_colors & !processed[state];
                if reads == 0 {
                    continue;
                }

                let defined = self.defined_reads[state]
                    | if state == usize::from(slot.0) {
                        1_u16 << slot.1
                    } else {
                        0
                    };
                if reads & !defined != 0 {
                    return false;
                }

                processed[state] |= reads;
                progressed = true;

                while reads != 0 {
                    let color = reads.trailing_zeros() as usize;
                    reads &= reads - 1;

                    let (print, _, target) = if state
                        == usize::from(slot.0)
                        && color == usize::from(slot.1)
                    {
                        *instr
                    } else {
                        *self
                            .prog
                            .get(&(state as State, color as Color))
                            .unwrap()
                    };

                    reachable_colors |= 1_u16 << print;
                    reachable_states |= 1_u16 << target;
                }
            }

            if !progressed {
                return true;
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

    fn branch(&mut self, config: Config) {
        self.branch_limited::<false>(config, None);
    }

    // The const flag keeps the regular DFS free of candidate-selection work.
    // Only the parallel third-instruction roots use the selected variant.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cognitive_complexity
    )]
    fn branch_limited<const SELECTED: bool>(
        &mut self,
        mut config: Config,
        only: Option<Instr>,
    ) {
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

        // When the rest of the tape is blank, the candidate leaves only its
        // printed color and 0 on the tape. Reject it if these colors cannot
        // lead to any undefined transition, even when rows for other colors
        // remain incomplete. Only try this after the cheaper pruning checks.
        if colors > 2
            && !self.final_slot()
            && config.tape.lspan.blank()
            && config.tape.rspan.blank()
        {
            // Direction does not affect this overapproximation, so reuse the
            // result for both shifts of the same (print, target) pair.
            let mut checked = [0_u16; colors];
            let mut terminal = [0_u16; colors];

            avail_instrs.retain(|instr @ &(print, _, target)| {
                let print_index = usize::from(print);
                if print_index >= usize::from(self.used_colors)
                    || usize::from(target)
                        >= usize::from(self.used_states)
                {
                    // A newly introduced color or state has undefined slots.
                    return true;
                }

                let target_bit = 1_u16 << target;
                if checked[print_index] & target_bit == 0 {
                    checked[print_index] |= target_bit;
                    if self.restricted_terminal(&slot, instr) {
                        terminal[print_index] |= target_bit;
                    }
                }

                terminal[print_index] & target_bit == 0
            });
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

        if SELECTED {
            let instr =
                only.expect("selected branch requires an instruction");
            avail_instrs.retain(|candidate| *candidate == instr);
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

            let shape = self.on_shape_insert(&slot, next_instr);
            self.instrs.on_insert(&slot, next_instr);

            self.branch(config.clone());

            self.instrs.on_remove();
            self.on_shape_remove(shape, &slot);
        }

        {
            self.prog.insert(&slot, last_instr);

            let shape = self.on_shape_insert(&slot, last_instr);
            self.instrs.on_insert(&slot, last_instr);

            self.branch(config);

            self.instrs.on_remove();
            self.on_shape_remove(shape, &slot);
        }

        self.prog.remove(&slot);

        self.remaining_slots += 1;
    }

    fn run_branch(
        init_instrs: &[Instr],
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
                tree.on_shape_insert(&INIT_SLOT, instr);
                tree.instrs.on_insert(&INIT_SLOT, instr);

                tree.branch(Config::init_stepped());

                (*instr, tree.harvester)
            })
            .collect()
    }

    // Run the selected B0 subtree in parallel at its next undefined slot.
    // Each third-instruction candidate receives independent Tree and Harvester
    // state, while all descendants continue through the normal sequential DFS.
    #[expect(clippy::iter_on_single_items, clippy::manual_let_else)]
    fn run_branch_second(
        second: Instr,
        halt: Slots,
        sim_lim: Steps,
        instr_table: &'i AvIn::Table,
        harvester: impl Send + Sync + Fn() -> Harv,
    ) -> TreeResult<Harv> {
        let mut probe = Self::init(
            halt,
            sim_lim,
            harvester(),
            AvIn::new(instr_table),
        );
        probe.remaining_slots -= 1;
        probe.prog.insert(&INIT_SLOT, &second);
        probe.on_shape_insert(&INIT_SLOT, &second);
        probe.instrs.on_insert(&INIT_SLOT, &second);

        // Normally B0 leads to another undefined slot. Handle roots that
        // instead terminate or reach the simulation limit without splitting.
        let mut config = Config::init_stepped();
        let slot = if let Undefined(slot) = probe.run(&mut config) {
            slot
        } else {
            probe.branch(Config::init_stepped());
            return [(second, probe.harvester)].into_iter().collect();
        };

        // Capture the normalized candidates after B0 changes the available
        // states/colors. branch_limited applies the full regular pruning to
        // each candidate before descending.
        let thirds = probe.instrs.avail_instrs(&slot).to_vec();
        drop(probe);

        thirds
            .par_iter()
            .map(|&third| {
                let mut tree = Self::init(
                    halt,
                    sim_lim,
                    harvester(),
                    AvIn::new(instr_table),
                );
                tree.remaining_slots -= 1;
                tree.prog.insert(&INIT_SLOT, &second);
                tree.on_shape_insert(&INIT_SLOT, &second);
                tree.instrs.on_insert(&INIT_SLOT, &second);

                tree.branch_limited::<true>(
                    Config::init_stepped(),
                    Some(third),
                );

                (third, tree.harvester)
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

    // Enumerate a single normalized subtree. A0 is fixed at 1RB, so
    // `second` is the instruction assigned to B0 (the first free slot).
    fn run_instrs_second<const instrs: usize>(
        second: &str,
        sim_lim: Steps,
        harvester: &(impl Send + Sync + Fn() -> Self),
    ) -> Self::Output {
        assert_eq!(states, instrs);
        assert_eq!(colors, instrs);

        let second = Instr::read(second);
        let (init_instrs, instr_table) =
            make_instr_table::<states, colors>();
        assert!(
            init_instrs.contains(&second),
            "invalid normalized B0 instruction: {}",
            second.show(),
        );

        let basic_runner = if states <= 3 && colors <= 3 {
            BasicTreeSmall::run_branch_second
        } else {
            BasicTree::run_branch_second
        };

        // Parallel tasks are the candidate instructions at the next undefined
        // slot (third instruction), rather than the already fixed B0.
        let results = basic_runner(
            second,
            (instrs * instrs) - instrs,
            sim_lim,
            &instr_table,
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
