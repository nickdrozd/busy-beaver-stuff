TEMPLATE = '''
#include <stdio.h>
#include <stdlib.h>

#define PROG "{}"

#define X_LIMIT 131072
#define TAPE_LEN ((X_LIMIT * 2) + 10)

#define CHECK_X do {{x_count++; if (x_count > X_LIMIT) {{goto H;}}}} while (0)

int POS = TAPE_LEN / 2;
int TAPE[TAPE_LEN];

#define ZERO_TAPE for (int i = 0; i < TAPE_LEN; i++) {{TAPE[i] = 0;}}

#define L POS--
#define R POS++

#define INSTRUCTION(c0, s0, t0, c1, s1, t1) \
  if (TAPE[POS]) {{TAPE[POS] = c1; s1; goto t1;}} else {{TAPE[POS] = c0; s0; goto t0;}}

int x_count = 0;
int a_count = 0;
int b_count = 0;
int c_count = 0;
int d_count = 0;

int main (void) {{
  ZERO_TAPE;

 A:
  CHECK_X; a_count = x_count;
  INSTRUCTION({});

 B:
  CHECK_X; b_count = x_count;
  INSTRUCTION({});

 C:
  CHECK_X; c_count = x_count;
  INSTRUCTION({});

 D:
  CHECK_X; d_count = x_count;
  INSTRUCTION({});

 H:
  printf("%s | %d %d %d %d\\n", PROG, a_count, b_count, c_count, d_count);
  exit(0);
}}
'''

def parse(program_string):
    split = iter(program_string.split())

    return (
        ','.join(x[0] + x[1])
        for x in zip(split, split)
    )


def main():
    prog = "1LB 1LA 0LC 1RA 1RC 1RD 0LB 0LD"

    A, B, C, D = parse(prog)

    print(TEMPLATE.format(prog, A, B, C, D))


if __name__ == '__main__':
    main()
