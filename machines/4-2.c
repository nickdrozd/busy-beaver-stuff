#include "machine.h"

#define STATES 4
#define COLORS 2

#define XLIMIT 35000000

int a1c, a1s, a1t,
  b0c, b0s, b0t, b1c, b1s, b1t,
  c0c, c0s, c0t, c1c, c1s, c1t,
  d0c, d0s, d0t, d1c, d1s, d1t;

#define LOAD_PROGRAM                            \
  READ_ACTION(a1c, a1s, a1t);                   \
  READ_ACTION(b0c, b0s, b0t);                   \
  READ_ACTION(b1c, b1s, b1t);                   \
  READ_ACTION(c0c, c0s, c0t);                   \
  READ_ACTION(c1c, c1s, c1t);                   \
  READ_ACTION(d0c, d0s, d0t);                   \
  READ_ACTION(d1c, d1s, d1t);                   \
  READ_BOUND;

SETUP;

int main (void) {
  DISPATCH_TABLE;

 INITIALIZE:
  RESET;
  LOAD_PROGRAM;

 A:
  INSTRUCTION(0, A0C, A0S, A0T, a1c, a1s, a1t);

 B:
  INSTRUCTION(1, b0c, b0s, b0t, b1c, b1s, b1t);

 C:
  INSTRUCTION(2, c0c, c0s, c0t, c1c, c1s, c1t);

 D:
  INSTRUCTION(3, d0c, d0s, d0t, d1c, d1s, d1t);

 H:
  WIPE_AND_SCORE;

  if (STEPS < XLIMIT)
    printf("%d | 1RB %c%c%c  %c%c%c %c%c%c  %c%c%c %c%c%c  %c%c%c %c%c%c | %d | %d\n",
           PROG_NUM,
           FORMAT_INSTR(a1c, a1s, a1t),
           FORMAT_INSTR(b0c, b0s, b0t),
           FORMAT_INSTR(b1c, b1s, b1t),
           FORMAT_INSTR(c0c, c0s, c0t),
           FORMAT_INSTR(c1c, c1s, c1t),
           FORMAT_INSTR(d0c, d0s, d0t),
           FORMAT_INSTR(d1c, d1s, d1t),
           STEPS,
           MARKS);

  goto INITIALIZE;

 EXIT:
  printf("done\n");
  exit(0);

 E:
 F:
 G:
  goto EXIT;
}
