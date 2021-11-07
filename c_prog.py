import sys

from tm import parse


def make_shift(shift):
    return 'RIGHT' if shift == 'R' else 'LEFT'


def make_binary_branch(instrs):
    (pr0, sh0, tr0), (pr1, sh1, tr1) = instrs

    return IF_TEMPLATE.format(
        'PRINT;' if pr0 == '1' else '',
        make_shift(sh0),
        tr0,

        'ERASE;' if pr1 == '0' else '',
        make_shift(sh1),
        tr1,
    )


IF_TEMPLATE = \
'''
  if (BLANK)
    {{
      {}
      {};
      goto {};
    }}
  else
    {{
      {}
      {};
      goto {};
    }}
'''


def make_n_ary_branch(instrs):
    return SWITCH_TEMPLATE.format(
        '\n'.join([
            make_case(color, instr)
            for color, instr in enumerate(instrs)
        ])
    )


SWITCH_TEMPLATE = \
'''
  switch (SCAN) {{
{}
  }}
'''


def make_case(color, instr):
    # pylint: disable = invalid-name
    pr, sh, tr = instr

    return CASE_TEMPLATE.format(color, pr, make_shift(sh), tr)


CASE_TEMPLATE = \
'''    case {}:
      WRITE({});
      {};
      goto {};'''


def make_branch(instrs):
    return (
        make_binary_branch
        if len(instrs) == 2 else
        make_n_ary_branch
    )(instrs)


def make_labels(prog):
    return '\n'.join([
        f' {chr(i + 65)}:' + make_branch(instrs)
        for i, instrs in
        enumerate(parse(prog))
    ])


def make_c(prog):
    return PROG_TEMPLATE.format(
        prog,
        make_labels(prog))


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


if __name__ == '__main__':
    for PROG in sys.stdin:
        print(
            make_c(
                PROG.strip()
            ).replace('      \n', '\n'))
