#include "machine.h"

#define X_LIMIT 33554432
#define BB_2_4 3932964
#define LOWER_BOUND BB_2_4
#define UPPER_BOUND 16777216

SETUP_TAPE;

#undef INSTRUCTION
#define INSTRUCTION(c0, s0, t0,                 \
                    c1, s1, t1,                 \
                    c2, s2, t2,                 \
                    c3, s3, t3)                 \
  if (SCAN(3)) ACTION(c3, s3, t3)               \
    else if (SCAN(2)) ACTION(c2, s2, t2)        \
      else if (SCAN(1)) ACTION(c1, s1, t1)      \
        else ACTION(c0, s0, t0)

unsigned int XX, AA, BB;
unsigned int PP = 0;

#define RESET_COUNTS XX = AA = BB = 0; PP++;

int a1c, a1s, a1t, a2c, a2s, a2t, a3c, a3s, a3t,
  b0c, b0s, b0t, b1c, b1s, b1t, b2c, b2s, b2t, b3c, b3s, b3t;

#define LOAD_PROGRAM                            \
  READ_ACTION(a1c, a1s, a1t);                   \
  READ_ACTION(a2c, a2s, a2t);                   \
  READ_ACTION(a3c, a3s, a3t);                   \
  READ_ACTION(b0c, b0s, b0t);                   \
  READ_ACTION(b1c, b1s, b1t);                   \
  READ_ACTION(b2c, b2s, b2t);                   \
  READ_ACTION(b3c, b3s, b3t);                   \
  READ_BOUND;

int main (void) {
  static void* dispatch[] = { &&A, &&B, &&C, &&D, &&E, &&F, &&G, &&H };

 INITIALIZE:
  RESET_COUNTS;
  RESET_TAPE;
  LOAD_PROGRAM;

 A:
  CHECK_X(AA);
  INSTRUCTION(A0C, A0S, A0T,
              a1c, a1s, a1t,
              a2c, a2s, a2t,
              a3c, a3s, a3t);

 B:
  CHECK_X(BB);
  INSTRUCTION(b0c, b0s, b0t,
              b1c, b1s, b1t,
              b2c, b2s, b2t,
              b3c, b3s, b3t);

 H:
  if (AA && BB)
    /* if (IN_RANGE(AA) || IN_RANGE(BB)) */
      printf("%d | 1RB %c%c%c %c%c%c %c%c%c %c%c%c %c%c%c %c%c%c %c%c%c | %d %d\n",
             PP,
             FORMAT_INSTR(a1c, a1s, a1t),
             FORMAT_INSTR(a2c, a2s, a2t),
             FORMAT_INSTR(a3c, a3s, a3t),
             FORMAT_INSTR(b0c, b0s, b0t),
             FORMAT_INSTR(b1c, b1s, b1t),
             FORMAT_INSTR(b2c, b2s, b2t),
             FORMAT_INSTR(b3c, b3s, b3t),
             AA, BB);

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
