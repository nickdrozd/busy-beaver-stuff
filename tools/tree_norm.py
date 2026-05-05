from tm.tape import Tape
from tools.normalize import Normalizer


def tree_swap(norm: Normalizer) -> bool:
    state = 0

    tape = Tape()

    avail_state, avail_color = 1, 1

    for _ in range(1_000):  # no-branch
        if state == avail_state:
            avail_state += 1
        elif state > avail_state:
            norm.swap_states(state, avail_state)
            return True

        if (scan := tape.scan) == avail_color:
            avail_color += 1
        elif scan > avail_color:
            norm.swap_colors(scan, avail_color)
            return True

        if (
            avail_state >= len(norm.states)
            and avail_color >= len(norm.colors)
        ):
            return False

        color, shift, next_state = norm[state, scan]

        tape.step(shift, color, state == next_state)

        state = next_state

    return False  # no-cover


def tree_norm(prog: str) -> str:
    norm = Normalizer(prog)

    for _ in range(1_000):  # no-branch
        if not tree_swap(norm):
            break

    return str(norm)
