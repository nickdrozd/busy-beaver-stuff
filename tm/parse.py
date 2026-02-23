from typing import TYPE_CHECKING

# ruff: noqa: F401
from tm.rust_stuff import tcompile

if TYPE_CHECKING:
    type Color = int
    type State = int
    type Shift = bool

    type Slot = tuple[State, Color]
    type Instr = tuple[Color, Shift, State]

    type Prog = dict[Slot, Instr]

    type Params = tuple[int, int]


def blank_loops(prog: Prog) -> dict[Shift, set[State]]:
    states: set[State] = set()

    for (s, _), (_, _, t) in prog.items():
        states.add(s)
        states.add(t)

    result: dict[Shift, set[State]] = {False: set(), True: set()}

    for sh in (False, True):
        nxt: dict[State, State] = {}
        for s in states:
            if (instr := prog.get((s, 0))) is None:
                continue
            _, ish, t = instr
            if ish == sh:
                nxt[s] = t

        color: dict[State, int] = dict.fromkeys(states, 0)

        for start in states:
            if color[start] != 0:
                continue

            cur = start
            stack: list[State] = []
            pos: dict[State, int] = {}

            while True:  # pylint: disable = while-used
                if color[cur] == 2:
                    for v in stack:
                        color[v] = 2
                    break

                if color[cur] == 1:
                    if (i := pos.get(cur)) is not None:  # no-branch
                        result[sh].update(stack[i:])
                    for v in stack:
                        color[v] = 2
                    break

                color[cur] = 1
                pos[cur] = len(stack)
                stack.append(cur)

                if cur not in nxt:
                    for v in stack:
                        color[v] = 2
                    break
                cur = nxt[cur]

    return result
