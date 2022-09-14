from tm import parse


def make_shift(shift):
    return 'RIGHT' if shift == 'R' else 'LEFT'


def make_branch(state, color, pr, sh, tr, indent):
    lines = [
        f'// {state}{color}',
        f'{make_shift(sh)};',
        f'goto {tr};',
    ]

    if color != (spr := int(pr)):
        lines.insert(
            1,
            'PRINT;' if spr == 1 else 'ERASE;')

    return ('\n' + (' ' * indent)).join(lines)


def make_if_else(state, instrs):
    (pr0, sh0, tr0), (pr1, sh1, tr1) = instrs

    return IF_TEMPLATE.format(
        make_branch(state, 0, pr0, sh0, tr0, 6),
        make_branch(state, 1, pr1, sh1, tr1, 6),
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


def make_case(state, color, instr):
    pr, sh, tr = instr

    return CASE_TEMPLATE.format(
        color,
        f'// {state}{color}',
        pr,
        make_shift(sh),
        tr,
    )


CASE_TEMPLATE = \
'''    case {}:
      {}
      WRITE({});
      {};
      goto {};'''


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
