#include "machine.h"

#define STATES 2
#define COLORS 2

#define X_LIMIT 40

int a0c, a0s, a0t, a1c, a1s, a1t, b0c, b0s, b0t, b1c, b1s, b1t;

#define LOAD_PROGRAM                            \
  READ_ACTION(a0c, a0s, a0t);                   \
  READ_ACTION(a1c, a1s, a1t);                   \
  READ_ACTION(b0c, b0s, b0t);                   \
  READ_ACTION(b1c, b1s, b1t);                   \
  READ_BOUND;

SETUP;

int main (void) {
  DISPATCH_TABLE;

 INITIALIZE:
  RESET_COUNTS;
  LOAD_PROGRAM;

 A:
  CHECK_X(AA);
  INSTRUCTION(a0c, a0s, a0t, a1c, a1s, a1t);

 B:
  CHECK_X(BB);
  INSTRUCTION(b0c, b0s, b0t, b1c, b1s, b1t);

 H:
  WIPE_AND_SCORE;

  printf("%d | %c%c%c %c%c%c  %c%c%c %c%c%c | %d %d | %d\n",
         PP,
         FORMAT_INSTR(a0c, a0s, a0t),
         FORMAT_INSTR(a1c, a1s, a1t),
         FORMAT_INSTR(b0c, b0s, b0t),
         FORMAT_INSTR(b1c, b1s, b1t),
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
