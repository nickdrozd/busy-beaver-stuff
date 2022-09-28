from typing import Optional, Tuple

from tm.parse import parse, st_str

Instr = Tuple[str, str, str]


def make_comment(st: str, co: int) -> str:
    return f'// {st}{co}'


def make_shift(sh: str) -> str:
    return (
        'RIGHT'
        if sh == 'R' else
        'LEFT'
    ) + ';'


def make_trans(tr: str) -> str:
    return f'goto {tr};'


def make_binary_write(pr: int) -> str:
    return (
        'PRINT'
        if pr == 1 else
        'ERASE'
    ) + ';'


def make_n_way_write(pr: int) -> str:
    return f'WRITE({pr});'


def make_instruction(
        st: str,
        co: int,
        pr: str,
        sh: str,
        tr: Optional[str],
        indent: int,
        binary: bool,
) -> str:
    lines = [
        make_comment(st, co),
        make_shift(sh),
    ]

    if co != (ipr := int(pr)):
        lines.insert(
            1,
            (
                make_binary_write
                if binary else
                make_n_way_write
            )(ipr)
        )

    if tr is not None:
        lines.append(
            make_trans(tr))

    return ('\n' + (' ' * indent)).join(lines)


def make_if_else(st: str, in0: Instr, in1: Instr) -> str:
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


def make_while(st: str, in0: Instr, in1: Instr) -> str:
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


def make_n_way_switch(state: str, instrs: Tuple[Instr, ...]) -> str:
    return SWITCH_TEMPLATE.format(
        '\n'.join([
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


def make_case(st: str, co: int, instr: Instr) -> str:
    return CASE_TEMPLATE.format(
        co,
        make_instruction(st, co, *instr, 6, False),
    )


CASE_TEMPLATE = \
'''    case {}:
      {}'''


def make_switch(state: str, instrs: Tuple[Instr, ...]) -> str:
    try:
        in0, in1 = instrs
    except ValueError:
        return make_n_way_switch(state, instrs)
    else:
        return make_if_else(state, in0, in1)


def make_labels(prog: str) -> str:
    return '\n'.join([
        (
            f' {st_str(i)}:'
            + make_switch(st_str(i), instrs)  # type: ignore
        )
        for i, instrs in
        enumerate(parse(prog))
    ])


def make_c(prog: str) -> str:
    return PROG_TEMPLATE.format(
        prog,
        make_labels(
            prog.replace('_', 'H')))


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
