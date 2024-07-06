from __future__ import annotations

from typing import TYPE_CHECKING

from tm.rust_stuff import read_instr

if TYPE_CHECKING:
    from tm.parse import Color, Instr, Params

    Switch = dict[Color, Instr | None]

########################################

def get_params(prog: str) -> Params:
    return len(parsed := parse(prog)), len(parsed[0])  # no-cover


def parse(prog: str) -> list[list[Instr | None]]:
    return [
        [
            read_instr(instr)
            for instr in instrs.split(' ')
        ]
        for instrs in prog.split('  ')
    ]
