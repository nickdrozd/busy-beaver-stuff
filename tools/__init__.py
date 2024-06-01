from __future__ import annotations

from typing import TYPE_CHECKING

from tm.rust_stuff import parse_to_vec as parse

if TYPE_CHECKING:
    from tm.parse import Color, Instr, Params

    Switch = dict[Color, Instr | None]

def get_params(prog: str) -> Params:
    return len(parsed := parse(prog)), len(parsed[0])
