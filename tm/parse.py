from typing import TYPE_CHECKING

# ruff:file-ignore[unused-import]
from tm.rust_stuff import tcompile

if TYPE_CHECKING:
    type Color = int
    type State = int
    type Shift = bool

    type Slot = tuple[State, Color]
    type Instr = tuple[Color, Shift, State]

    type Prog = dict[Slot, Instr]

    type Params = tuple[int, int]


def _states_colors(
        prog: Prog,
) -> tuple[set[State], tuple[Color, ...]]:
    states: set[State] = set()
    colors: set[Color] = {0}

    for (state, color), (write, _, next_state) in prog.items():
        states.add(state)
        states.add(next_state)
        colors.add(color)
        colors.add(write)

    return states, tuple(sorted(colors))


def _monotone_blank_loops(
        prog: Prog,
        states: set[State],
) -> dict[Shift, set[State]]:
    result: dict[Shift, set[State]] = {False: set(), True: set()}

    for shift in (False, True):
        nxt: dict[State, State] = {}
        rev: dict[State, list[State]] = {}

        for state in states:
            instr = prog.get((state, 0))

            if instr is None:
                continue

            _, instr_shift, next_state = instr

            if instr_shift != shift:
                continue

            nxt[state] = next_state
            rev.setdefault(next_state, []).append(state)

        bad = states - nxt.keys()
        stack = list(bad)

        while stack:
            next_state = stack.pop()

            for state in rev.get(next_state, ()):
                if state in bad:  # no-cover
                    continue

                bad.add(state)
                stack.append(state)

        result[shift].update(nxt.keys() - bad)

    return result


def _write_symbol(
        tape: dict[int, Color],
        pos: int,
        color: Color,
) -> None:
    if pos < 0 or color != 0:
        tape[pos] = color
    else:
        tape.pop(pos, None)


def _same_suffix(
        before: dict[int, Color],
        after: dict[int, Color],
        cutoff: int,
        delta: int,
) -> bool:
    translated = {
        pos + delta: color
        for pos, color in before.items()
        if pos >= cutoff
    }
    actual = {
        pos: color
        for pos, color in after.items()
        if pos >= cutoff + delta
    }

    return translated == actual


def _tape_key(tape: dict[int, Color]) -> tuple[tuple[int, Color], ...]:
    return tuple(sorted(tape.items()))


def _expand(
        prog: Prog,
        colors: tuple[Color, ...],
        outward: Shift,
        state: State,
        head: int,
        tape: dict[int, Color],
        cache: dict[
            tuple[Shift, State, int, tuple[tuple[int, Color], ...]],
            tuple[tuple[State, int, tuple[tuple[int, Color], ...]], ...] | None,
        ],
) -> tuple[tuple[State, int, tuple[tuple[int, Color], ...]], ...] | None:
    key = outward, state, head, _tape_key(tape)

    if key in cache:
        return cache[key]

    known = head >= 0 or head in tape

    if known:
        color = tape.get(head, 0)
        instr = prog.get((state, color))

        if instr is None:
            cache[key] = None
            return None

        write, shift, next_state = instr
        next_tape = tape.copy()
        _write_symbol(next_tape, head, write)
        result: tuple[tuple[State, int, tuple[tuple[int, Color], ...]], ...] = ((
            next_state,
            head + (1 if shift == outward else -1),
            _tape_key(next_tape),
        ),)
        cache[key] = result
        return result

    instructions: set[tuple[Color, Shift, State]] = set()

    for color in colors:
        instr = prog.get((state, color))

        if instr is None:
            cache[key] = None
            return None

        instructions.add(instr)

    branches: list[tuple[State, int, tuple[tuple[int, Color], ...]]] = []

    for write, shift, next_state in instructions:
        next_tape = tape.copy()
        _write_symbol(next_tape, head, write)
        branches.append((
            next_state,
            head + (1 if shift == outward else -1),
            _tape_key(next_tape),
        ))

    result = tuple(branches)
    cache[key] = result
    return result


def _blank_side_spinout(
        prog: Prog,
        colors: tuple[Color, ...],
        start: State,
        outward: Shift,
        expand_cache: dict[
            tuple[Shift, State, int, tuple[tuple[int, Color], ...]],
            tuple[tuple[State, int, tuple[tuple[int, Color], ...]], ...] | None,
        ],
        sim_lim: int = 256,
) -> bool:
    nodes_left = sim_lim * 32

    def prove(
            state: State,
            head: int,
            tape_key: tuple[tuple[int, Color], ...],
            history: tuple[
                tuple[State, int, tuple[tuple[int, Color], ...]],
                ...,
            ],
            depth: int,
    ) -> bool:
        nonlocal nodes_left

        if depth >= sim_lim or nodes_left <= 0:
            return False

        nodes_left -= 1
        tape = dict(tape_key)
        min_head = head

        for old_state, old_head, old_tape_key in reversed(history):
            min_head = min(min_head, old_head)

            if old_state != state:
                continue

            delta = head - old_head

            if delta < 0:
                continue

            if delta > 0 and min_head + delta < 0:
                continue

            if _same_suffix(dict(old_tape_key), tape, min_head, delta):
                return True

        branches = _expand(
            prog,
            colors,
            outward,
            state,
            head,
            tape,
            expand_cache,
        )

        if branches is None:
            return False

        next_history = (*history, (state, head, tape_key))

        for next_state, next_head, next_tape_key in branches:
            if not prove(
                    next_state,
                    next_head,
                    next_tape_key,
                    next_history,
                    depth + 1,
            ):
                return False

        return True

    return prove(start, 0, (), (), 0)


def blank_loops(prog: Prog) -> dict[Shift, set[State]]:
    states, colors = _states_colors(prog)
    result = _monotone_blank_loops(prog, states)
    expand_cache: dict[
        tuple[Shift, State, int, tuple[tuple[int, Color], ...]],
        tuple[tuple[State, int, tuple[tuple[int, Color], ...]], ...] | None,
    ] = {}

    for outward in (False, True):
        for state in states:
            if state in result[outward]:
                continue

            instr = prog.get((state, 0))

            if instr is None or instr[1] != outward:
                continue

            if _blank_side_spinout(
                    prog,
                    colors,
                    state,
                    outward,
                    expand_cache,
            ):
                result[outward].add(state)

    return result
