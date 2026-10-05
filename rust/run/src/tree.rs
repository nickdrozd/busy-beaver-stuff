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

struct Tree<const states: usize, const colors: usize, AvIn, Harv> {
    prog: Prog<states, colors>,
    instrs: AvIn,
    sim_lim: Steps,
    remaining_slots: Slots,
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

        Self {
            prog,
            instrs,
            sim_lim,
            remaining_slots,
            harvester,
        }
    }

    const fn final_slot(&self) -> bool {
        self.remaining_slots == 0
    }

    fn run(&self, config: &mut Config) -> RunResult {
        self.prog.run_basic(self.sim_lim, config)
    }

    fn is_nontrivial(&self, config: &mut Config) -> bool {
        matches!(self.run(config), StepLimit)
    }

    fn harvest(&mut self, config: &mut PassConfig<'_>) {
        self.harvester.harvest(&self.prog, config);
    }

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

        if config.tape.scan == 0 {
            avail_instrs.retain(|&(_, shift, state)| {
                !(config.state == state && config.tape.at_edge(shift))
            });
        } else if config.tape.lspan.blank() && config.tape.rspan.blank()
        {
            avail_instrs.retain(|&(pr, _, _)| pr != 0);
        }

        avail_instrs.retain(|instr| self.instrs.allows(&slot, instr));

        let Some((last_instr, instrs)) = avail_instrs.split_last()
        else {
            return;
        };

        if self.final_slot() {
            for next_instr in instrs {
                self.prog.insert(&slot, next_instr);

                self.harvest(&mut PassConfig::Borrowed(&config));
            }

            {
                self.prog.insert(&slot, last_instr);

                if self.is_nontrivial(&mut config) {
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

            self.instrs.on_insert(&slot, next_instr);

            self.branch(config.clone());

            self.instrs.on_remove();
        }

        {
            self.prog.insert(&slot, last_instr);

            self.instrs.on_insert(&slot, last_instr);

            self.branch(config);

            self.instrs.on_remove();
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
