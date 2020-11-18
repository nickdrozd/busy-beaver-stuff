#include "machine.h"

#define X_LIMIT 134217728  // 2^27
#define TAPE_LEN ((X_LIMIT * 2) + 10)
#define BB5_STEPS 47176870
#define UPPER_BOUND 100663296  // 2^27 - 2^25

#undef IN_RANGE
#define IN_RANGE(COUNT) (COUNT < UPPER_BOUND)

unsigned int POS;
unsigned int TAPE[TAPE_LEN];

#define INSTRUCTION(c0, s0, t0, c1, s1, t1)     \
  if (SCAN(1))                                  \
    ACTION(c1, s1, t1)                          \
    else                                        \
      ACTION(c0, s0, t0)

unsigned int XX, AA, BB, CC, DD, EE;
unsigned int PP = 0;

#define RESET_COUNTS XX = AA = BB = CC = DD = EE = 0; PP++;

int a1c, a1s, a1t,
  b0c, b0s, b0t, b1c, b1s, b1t,
  c0c, c0s, c0t, c1c, c1s, c1t,
  d0c, d0s, d0t, d1c, d1s, d1t,
  e0c, e0s, e0t, e1c, e1s, e1t;

#define LOAD_PROGRAM                            \
  READ_ACTION(a1c, a1s, a1t);                   \
  READ_ACTION(b0c, b0s, b0t);                   \
  READ_ACTION(b1c, b1s, b1t);                   \
  READ_ACTION(c0c, c0s, c0t);                   \
  READ_ACTION(c1c, c1s, c1t);                   \
  READ_ACTION(d0c, d0s, d0t);                   \
  READ_ACTION(d1c, d1s, d1t);                   \
  READ_ACTION(e0c, e0s, e0t);                   \
  READ_ACTION(e1c, e1s, e1t);                   \
  READ_BOUND;

int main (void) {
  static void* dispatch[] = { &&A, &&B, &&C, &&D, &&E, &&F, &&G, &&H };

 INITIALIZE:
  RESET_COUNTS;
  RESET_TAPE;
  LOAD_PROGRAM;

 A:
  CHECK_X(AA);
  INSTRUCTION(A0C, A0S, A0T, a1c, a1s, a1t);

 B:
  CHECK_X(BB);
  INSTRUCTION(b0c, b0s, b0t, b1c, b1s, b1t);

 C:
  CHECK_X(CC);
  INSTRUCTION(c0c, c0s, c0t, c1c, c1s, c1t);

 D:
  CHECK_X(DD);
  INSTRUCTION(d0c, d0s, d0t, d1c, d1s, d1t);

 E:
  CHECK_X(EE);
  INSTRUCTION(e0c, e0s, e0t, e1c, e1s, e1t);

 H:
  if (AA && BB && CC && DD && EE)
    if (IN_RANGE(AA) || IN_RANGE(BB) || IN_RANGE(CC) || IN_RANGE(DD) || IN_RANGE(EE))
      printf("%d | 1RB %c%c%c %c%c%c %c%c%c %c%c%c %c%c%c %c%c%c %c%c%c %c%c%c %c%c%c | %d %d %d %d %d\n",
             PP,
             FORMAT_INSTR(a1c, a1s, a1t),
             FORMAT_INSTR(b0c, b0s, b0t),
             FORMAT_INSTR(b1c, b1s, b1t),
             FORMAT_INSTR(c0c, c0s, c0t),
             FORMAT_INSTR(c1c, c1s, c1t),
             FORMAT_INSTR(d0c, d0s, d0t),
             FORMAT_INSTR(d1c, d1s, d1t),
             FORMAT_INSTR(e0c, e0s, e0t),
             FORMAT_INSTR(e1c, e1s, e1t),
             AA, BB, CC, DD, EE);

  goto INITIALIZE;

 EXIT:
  printf("done\n");
  exit(0);

 F:
 G:
  goto EXIT;
}
