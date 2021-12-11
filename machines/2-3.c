#include "machine.h"

#define STATES 2
#define COLORS 3

#define X_LIMIT 300

#undef INSTRUCTION
#define INSTRUCTION(c0, s0, t0,                 \
                    c1, s1, t1,                 \
                    c2, s2, t2)                 \
  if (SCAN(2)) ACTION(c2, s2, t2)               \
    else if (SCAN(1)) ACTION(c1, s1, t1)        \
      else ACTION(c0, s0, t0)

int a1c, a1s, a1t, a2c, a2s, a2t,
  b0c, b0s, b0t, b1c, b1s, b1t, b2c, b2s, b2t;

#define LOAD_PROGRAM                            \
  READ_ACTION(a1c, a1s, a1t);                   \
  READ_ACTION(a2c, a2s, a2t);                   \
  READ_ACTION(b0c, b0s, b0t);                   \
  READ_ACTION(b1c, b1s, b1t);                   \
  READ_ACTION(b2c, b2s, b2t);                   \
  READ_BOUND;

SETUP;

int main (void) {
  DISPATCH_TABLE;

 INITIALIZE:
  RESET_COUNTS;
  LOAD_PROGRAM;

 A:
  CHECK_LIMIT(AA);
  INSTRUCTION(A0C, A0S, A0T, a1c, a1s, a1t, a2c, a2s, a2t);

 B:
  CHECK_LIMIT(BB);
  INSTRUCTION(b0c, b0s, b0t, b1c, b1s, b1t, b2c, b2s, b2t);

 H:
  WIPE_AND_SCORE;

  printf("%d | 1RB %c%c%c %c%c%c  %c%c%c %c%c%c %c%c%c | %d %d | %d\n",
         PP,
         FORMAT_INSTR(a1c, a1s, a1t),
         FORMAT_INSTR(a2c, a2s, a2t),
         FORMAT_INSTR(b0c, b0s, b0t),
         FORMAT_INSTR(b1c, b1s, b1t),
         FORMAT_INSTR(b2c, b2s, b2t),
         AA, BB,
         MARKS);

  goto INITIALIZE;

 EXIT:
  printf("done\n");
  exit(0);

 C:
 D:
 E:
 F:
 G:
  goto EXIT;
}
