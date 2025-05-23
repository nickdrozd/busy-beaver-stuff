# ruff: noqa: Q001
from typing import TYPE_CHECKING

from tm.show import show_state
from tools import parse

if TYPE_CHECKING:
    from tm.parse import Color, Instr, Shift, State
    from tools import Switch


def make_comment(st: State, co: Color) -> str:
    return f'// {show_state(st)}{co}'


def make_shift(sh: Shift) -> str:
    return ('RIGHT' if sh else 'LEFT') + ';'


def make_trans(tr: State) -> str:
    return f'goto {show_state(tr) if 0 <= tr else "_"};'


def make_binary_write(pr: Color) -> str:
    return ('PRINT' if pr == 1 else 'ERASE') + ';'


def make_n_way_write(pr: Color) -> str:
    return f'WRITE({pr});'


def make_instruction(
        st: State,
        co: Color,
        instr: Instr,
        *,
        binary: bool = True,
        skip_trans: bool = False,
) -> list[str]:
    pr, sh, tr = instr

    lines = [
        make_comment(st, co),
        make_shift(sh),
    ]

    if co != pr:
        lines.insert(
            1,
            (
                make_binary_write
                if binary else
                make_n_way_write
            )(pr)
        )

    if not skip_trans:
        lines.append(
            make_trans(tr))

    return lines


def indent(space: int, lines: list[str]) -> str:
    return ('\n' + (' ' * space)).join(lines)


def make_if_else(st: State, in0: Instr, in1: Instr) -> str:
    _, _, tr0 = in0
    _, _, tr1 = in1

    return (
        make_while(st, in0, in1, mark = False) if st == tr0 else
        make_while(st, in1, in0, mark =  True) if st == tr1 else
        IF_TEMPLATE.format(
            indent(6, make_instruction(st, 0, in0)),
            indent(6, make_instruction(st, 1, in1)),
        )
    )


IF_TEMPLATE = \
'''
  if (BLANK)
    {{
      {}
    }}
  else
    {{
      {}
    }}
'''


def make_while(
        st: State,
        loop_in: Instr,
        rest_in: Instr,
        mark: bool,  # noqa: FBT001
) -> str:
    test = 'BLANK' if not mark else '!BLANK'
    loop = make_instruction(st, int(mark), loop_in, skip_trans = True)
    rest = make_instruction(st, 1 - int(mark), rest_in)

    return WHILE_TEMPLATE.format(
        test,
        indent(4, loop),
        indent(2, rest),
    )


WHILE_TEMPLATE = \
'''
  while ({}) {{
    {}
  }}

  {}
'''


def make_n_way_switch(state: State, instrs: tuple[Instr, ...]) -> str:
    return SWITCH_TEMPLATE.format(
        '\n\n'.join([
            make_case(state, color, instr)
            for color, instr in enumerate(instrs)
        ])
    )


SWITCH_TEMPLATE = \
'''
  switch (SCAN) {{
{}
  }}
'''


def make_case(st: State, co: Color, instr: Instr) -> str:
    return CASE_TEMPLATE.format(
        co,
        indent(6, make_instruction(st, co, instr, binary = False)),
    )


CASE_TEMPLATE = \
'''    case {}:
      {}'''


def make_switch(state: State, instrs: tuple[Instr, ...]) -> str:
    try:
        in0, in1 = instrs
    except ValueError:
        return make_n_way_switch(state, instrs)

    return make_if_else(state, in0, in1)


def make_label(state: State) -> str:
    return f' {show_state(state)}:'


def parse_prog(prog: str) -> list[tuple[State, Switch]]:
    return sorted([
        (state, dict(enumerate(instrs)))
        for state, instrs in enumerate(parse(prog))
    ])


def make_labels(prog: str) -> str:
    return '\n'.join([
        make_label(state) + make_switch(
            state,
            tuple(
                instr or (1, True, -1)
                for instr in switch.values()),
        )
        for state, switch in parse_prog(prog)
    ])


def make_c(prog: str) -> str:
    return PROG_TEMPLATE.format(
        prog,
        make_labels(prog),
    ).replace(' _', ' H')


PROG_TEMPLATE = \
'''#define PROGRAM "{}"

#define TAPELEN 100

#include "machine.h"

int main(void)
{{
{}
 H:
  CHECK_STEPS;
}}'''
