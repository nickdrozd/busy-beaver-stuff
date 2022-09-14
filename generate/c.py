from typing import Optional

from tm import parse


def make_comment(st, co):
    return f'// {st}{co}'


def make_shift(sh):
    return (
        'RIGHT'
        if sh == 'R' else
        'LEFT'
    ) + ';'


def make_trans(tr):
    return f'goto {tr};'


def make_binary_write(pr: int):
    return (
        'PRINT'
        if pr == 1 else
        'ERASE'
    ) + ';'


def make_n_way_write(pr):
    return f'WRITE({pr});'


def make_instruction(
        st: str,
        co: int,
        pr: str,
        sh: str,
        tr: Optional[str],
        indent: int,
        binary: bool,
):
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


def make_if_else(st, instrs):
    (pr0, sh0, tr0), (pr1, sh1, tr1) = instrs

    if st in (tr0, tr1):
        return make_while(st, instrs)

    return IF_TEMPLATE.format(
        make_instruction(st, 0, pr0, sh0, tr0, 6, True),
        make_instruction(st, 1, pr1, sh1, tr1, 6, True),
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


def make_while(st, instrs):
    (pr0, sh0, tr0), (pr1, sh1, _) = in0, in1 = instrs

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


def make_n_way_switch(state, instrs):
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


def make_case(st, co, instr):
    pr, sh, tr = instr

    return CASE_TEMPLATE.format(
        co,
        make_instruction(st, co, pr, sh, tr, 6, False),
    )


CASE_TEMPLATE = \
'''    case {}:
      {}'''


def make_switch(state, instrs):
    return (
        make_if_else
        if len(instrs) == 2 else
        make_n_way_switch
    )(state, instrs)


def make_labels(prog):
    return '\n'.join([
        f' {chr(i + 65)}:' + make_switch(chr(i + 65), instrs)
        for i, instrs in
        enumerate(parse(prog))
    ])


def make_c(prog):
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
