#include <stdio.h>
#include <stdlib.h>

#define X_LIMIT 134217728
#define TAPE_LEN ((X_LIMIT * 2) + 10)
#define BB5_STEPS 47176870
#define UPPER_BOUND 100000000

#define IN_RANGE(COUNT) (BB5_STEPS < COUNT && COUNT < UPPER_BOUND)

#define CHECK_X(COUNT) {                        \
    XX++;                                       \
    if (XX > X_LIMIT) {goto H;};                \
    COUNT = XX;                                 \
  }

unsigned int POS;
unsigned int TAPE[TAPE_LEN];

#define RESET_TAPE                              \
  POS = TAPE_LEN / 2;                           \
  for (int i = 0; i < TAPE_LEN; i++) {          \
    TAPE[i] = 0;                                \
  }

#define L POS--;
#define R POS++;

#define ACTION(c, s, t) {                       \
    TAPE[POS] = c - 48;                         \
    if (s - 76) { R } else { L };               \
    goto *dispatch[t - 65];                     \
  }

#define INSTRUCTION(c0, s0, t0, c1, s1, t1)     \
  if (TAPE[POS])                                \
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

#define READ(VAR) if ((VAR = getc(stdin)) == EOF) goto EXIT;

#define READ_ACTION(C, S, T) READ(C); READ(S); READ(T);

#define LOAD_PROGRAM                                        \
  READ_ACTION(a1c, a1s, a1t);                               \
  READ_ACTION(b0c, b0s, b0t); READ_ACTION(b1c, b1s, b1t);   \
  READ_ACTION(c0c, c0s, c0t); READ_ACTION(c1c, c1s, c1t);   \
  READ_ACTION(d0c, d0s, d0t); READ_ACTION(d1c, d1s, d1t);   \
  READ_ACTION(e0c, e0s, e0t); READ_ACTION(e1c, e1s, e1t);   \
  getc(stdin);

int main (void) {
  static void* dispatch[] = { &&A, &&B, &&C, &&D, &&E, &&F, &&G, &&H };

 INITIALIZE:
  RESET_COUNTS;
  RESET_TAPE;
  LOAD_PROGRAM;

 A:
  CHECK_X(AA);
  INSTRUCTION('1', 'R', 'B', a1c, a1s, a1t);

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
  /* if (AA && BB && CC && DD && EE) */
  /*   if (IN_RANGE(AA) || IN_RANGE(BB) || IN_RANGE(CC) || IN_RANGE(DD) || IN_RANGE(EE)) */
      printf("%d | 1RB %c%c%c %c%c%c %c%c%c %c%c%c %c%c%c %c%c%c %c%c%c %c%c%c %c%c%c | %d %d %d %d %d\n",
             PP,
             a1c, a1s, a1t,
             b0c, b0s, b0t, b1c, b1s, b1t,
             c0c, c0s, c0t, c1c, c1s, c1t,
             d0c, d0s, d0t, d1c, d1s, d1t,
             e0c, e0s, e0t, e1c, e1s, e1t,
             AA, BB, CC, DD, EE);

  goto INITIALIZE;

 EXIT:
  printf("done\n");
  exit(0);

 F:
 G:
  goto EXIT;
}
