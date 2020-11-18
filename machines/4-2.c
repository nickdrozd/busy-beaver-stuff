#include "machine.h"

#define X_LIMIT 2097152
#define TAPE_LEN ((X_LIMIT * 2) + 10)
#define BB4 107
#define LOWER_BOUND BB4
#define UPPER_BOUND (X_LIMIT / 2)

unsigned int POS;
unsigned int TAPE[TAPE_LEN];

unsigned int XX, AA, BB, CC, DD;
unsigned int PP = 0;

#define RESET_COUNTS XX = AA = BB = CC = DD = 0; PP++;

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

 H:
  if (IN_RANGE(AA) || IN_RANGE(BB) || IN_RANGE(CC) || IN_RANGE(DD))
    printf("%d | 1RB %c%c%c %c%c%c %c%c%c %c%c%c %c%c%c %c%c%c %c%c%c | %d %d %d %d\n",
           PP,
           FORMAT_INSTR(a1c, a1s, a1t),
           FORMAT_INSTR(b0c, b0s, b0t),
           FORMAT_INSTR(b1c, b1s, b1t),
           FORMAT_INSTR(c0c, c0s, c0t),
           FORMAT_INSTR(c1c, c1s, c1t),
           FORMAT_INSTR(d0c, d0s, d0t),
           FORMAT_INSTR(d1c, d1s, d1t),
           AA, BB, CC, DD);

  goto INITIALIZE;

 EXIT:
  printf("done\n");
  exit(0);

 E:
 F:
 G:
  goto EXIT;
}
