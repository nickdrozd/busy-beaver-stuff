#include <stdio.h>
#include <stdlib.h>

#define X_LIMIT 300
#define TAPE_LEN ((X_LIMIT * 2) + 10)
#define LOWER_BOUND 30
#define UPPER_BOUND 100

#define IN_RANGE(COUNT) (LOWER_BOUND <= COUNT && COUNT < UPPER_BOUND)

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

#define INSTRUCTION(c0, s0, t0, c1, s1, t1, c2, s2, t2) \
  if (TAPE[POS] == 2)                                   \
    ACTION(c2, s2, t2)                                  \
    else if (TAPE[POS] == 1)                            \
      ACTION(c1, s1, t1)                                \
      else                                              \
        ACTION(c0, s0, t0)

unsigned int XX, AA, BB;
unsigned int PP = 0;

#define RESET_COUNTS XX = AA = BB = 0; PP++;

int a1c, a1s, a1t, a2c, a2s, a2t,
  b0c, b0s, b0t, b1c, b1s, b1t, b2c, b2s, b2t;

#define READ(VAR) if ((VAR = getc(stdin)) == EOF) goto EXIT;

#define READ_ACTION(C, S, T) READ(C); READ(S); READ(T);

#define LOAD_PROGRAM                            \
  READ_ACTION(a1c, a1s, a1t);                   \
  READ_ACTION(a2c, a2s, a2t);                   \
  READ_ACTION(b0c, b0s, b0t);                   \
  READ_ACTION(b1c, b1s, b1t);                   \
  READ_ACTION(b2c, b2s, b2t);                   \
  getc(stdin);

int main (void) {
  static void* dispatch[] = { &&A, &&B, &&C, &&D, &&E, &&F, &&G, &&H };

 INITIALIZE:
  RESET_COUNTS;
  RESET_TAPE;
  LOAD_PROGRAM;

 A:
  CHECK_X(AA);
  INSTRUCTION('1', 'R', 'B', a1c, a1s, a1t, a2c, a2s, a2t);

 B:
  CHECK_X(BB);
  INSTRUCTION(b0c, b0s, b0t, b1c, b1s, b1t, b2c, b2s, b2t);

 H:
  if (AA && BB)
    if (IN_RANGE(AA) || IN_RANGE(BB))
      printf("%d | 1RB %c%c%c %c%c%c %c%c%c %c%c%c %c%c%c | %d %d\n",
             PP,
             a1c, a1s, a1t, a2c, a2s, a2t,
             b0c, b0s, b0t, b1c, b1s, b1t, b2c, b2s, b2t,
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
