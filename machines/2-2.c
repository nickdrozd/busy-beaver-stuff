#include <stdio.h>
#include <stdlib.h>

#define X_LIMIT 40
#define UPPER_BOUND (X_LIMIT / 2)
#define LOWER_BOUND 2
#define TAPE_LEN ((X_LIMIT * 2) + 10)

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
    TAPE[POS] = c;                              \
    if (s) { R } else { L };                    \
    goto *dispatch[t];                          \
  }

#define INSTRUCTION(c0, s0, t0, c1, s1, t1)     \
  if (TAPE[POS] == 1)                           \
    ACTION(c1, s1, t1)                          \
    else                                        \
      ACTION(c0, s0, t0)

unsigned int XX, AA, BB;
unsigned int PP = 0;

#define RESET_COUNTS XX = AA = BB = 0; PP++;

int a0c, a0s, a0t, a1c, a1s, a1t, b0c, b0s, b0t, b1c, b1s, b1t;

#define READ(VAR) if ((VAR = getc(stdin)) == EOF) goto EXIT;

#define COLOR_CONV '0'
#define SHIFT_CONV 'L'
#define TRANS_CONV 'A'

#define READ_ACTION(C, S, T) {                  \
    READ(C); C -= COLOR_CONV;                   \
    READ(S); S -= SHIFT_CONV;                   \
    READ(T); T -= TRANS_CONV;                   \
  }

#define LOAD_PROGRAM                            \
  READ_ACTION(a0c, a0s, a0t);                   \
  READ_ACTION(a1c, a1s, a1t);                   \
  READ_ACTION(b0c, b0s, b0t);                   \
  READ_ACTION(b1c, b1s, b1t);                   \
  getc(stdin);

#define FORMAT_INSTR(C, S, T)                       \
  C + COLOR_CONV, S + SHIFT_CONV, T + TRANS_CONV

int main (void) {
  static void* dispatch[] = { &&A, &&B, &&C, &&D, &&E, &&F, &&G, &&H };

 INITIALIZE:
  RESET_COUNTS;
  RESET_TAPE;
  LOAD_PROGRAM;

 A:
  CHECK_X(AA);
  INSTRUCTION(a0c, a0s, a0t, a1c, a1s, a1t);

 B:
  CHECK_X(BB);
  INSTRUCTION(b0c, b0s, b0t, b1c, b1s, b1t);

 H:
  if (AA && BB)
    if (IN_RANGE(AA) || IN_RANGE(BB))
      printf("%d | %c%c%c %c%c%c %c%c%c %c%c%c | %d %d\n",
             PP,
             FORMAT_INSTR(a0c, a0s, a0t),
             FORMAT_INSTR(a1c, a1s, a1t),
             FORMAT_INSTR(b0c, b0s, b0t),
             FORMAT_INSTR(b1c, b1s, b1t),
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
