from tm.rust_stuff import st_str
from tm.program import Program
from tm.instrs import Color, Shift, State, Instr


def make_comment(st: State, co: Color) -> str:
    return f'// {st_str(st)}{co}'


def make_shift(sh: Shift) -> str:
    return ('RIGHT' if sh else 'LEFT') + ';'


def make_trans(tr: State) -> str:
    return f'goto {st_str(tr)};'


def make_binary_write(pr: Color) -> str:
    return ('PRINT' if pr == 1 else 'ERASE') + ';'


def make_n_way_write(pr: Color) -> str:
    return f'WRITE({pr});'


def make_instruction(
        st: State,
        co: Color,
        pr: Color,
        sh: Shift,
        tr: State | None,
        indent: int,
        binary: bool,
) -> str:
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

    if tr is not None:
        lines.append(
            make_trans(tr))

    return ('\n' + (' ' * indent)).join(lines)


def make_if_else(st: State, in0: Instr, in1: Instr) -> str:
    _, _, tr0 = in0
    _, _, tr1 = in1

    if st in (tr0, tr1):
        return make_while(st, in0, in1)

    return IF_TEMPLATE.format(
        make_instruction(st, 0, *in0, 6, True),
        make_instruction(st, 1, *in1, 6, True),
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


def make_while(st: State, in0: Instr, in1: Instr) -> str:
    pr0, sh0, tr0 = in0
    pr1, sh1,   _ = in1

    if tr0 == st:
        test = 'BLANK'
        loop = make_instruction(st, 0, pr0, sh0, None, 4, True)
        rest = make_instruction(st, 1, *in1, 2, True)
    else:
        test = '!BLANK'
        loop = make_instruction(st, 1, pr1, sh1, None, 4, True)
        rest = make_instruction(st, 0, *in0, 2, True)

    return WHILE_TEMPLATE.format(test, loop, rest)


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
        make_instruction(st, co, *instr, 6, False),
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
    return f' {st_str(state)}:'


def make_labels(prog: str) -> str:
    return '\n'.join([
        make_label(state) + make_switch(
            state,
            tuple(
                instr or (1, True, -1)
                for instr in switch.values()),
        )
        for state, switch in Program(prog).state_switches
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
