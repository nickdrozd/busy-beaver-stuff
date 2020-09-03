#include <stdio.h>
#include <stdlib.h>

#define X_LIMIT 100000
#define TAPE_LEN ((X_LIMIT * 2) + 10)

#define CHECK_X(COUNT) do {                     \
    XX++;                                       \
    if (XX > X_LIMIT) {goto H;};                \
    COUNT = XX;                                 \
  } while (0)

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

unsigned int XX, AA, BB, CC, DD;

#define RESET_COUNTS XX = AA = BB = CC = DD = 0;

int c0, c1, c2, c3, c4, c5, c6, c7,
  c8, c9, c10, c11, c12, c13, c14, c15,
  c16, c17, c18, c19, c20, c21, c22, c23;

#define READ(VAR) if ((VAR = getc(stdin)) == EOF) goto EXIT;

#define LOAD_PROGRAM                            \
  READ(c0); READ(c1); READ(c2); READ(c3);       \
  READ(c4); READ(c5); READ(c6); READ(c7);       \
  READ(c8); READ(c9); READ(c10); READ(c11);     \
  READ(c12); READ(c13); READ(c14); READ(c15);   \
  READ(c16); READ(c17); READ(c18); READ(c19);   \
  READ(c20); READ(c21); READ(c22); READ(c23);   \
  getc(stdin);

int main (void) {
  static void* dispatch[] = { &&A, &&B, &&C, &&D, &&E, &&F, &&G, &&H };

 INITIALIZE:
  RESET_COUNTS;
  RESET_TAPE;
  LOAD_PROGRAM;

 A:
  CHECK_X(AA);
  INSTRUCTION(c0, c1, c2, c3, c4, c5);

 B:
  CHECK_X(BB);
  INSTRUCTION(c6, c7, c8, c9, c10, c11);

 C:
  CHECK_X(CC);
  INSTRUCTION(c12, c13, c14, c15, c16, c17);

 D:
  CHECK_X(DD);
  INSTRUCTION(c18, c19, c20, c21, c22, c23);

 H:
  printf("%c%c%c %c%c%c %c%c%c %c%c%c %c%c%c %c%c%c %c%c%c %c%c%c | %d %d %d %d\n",
         c0, c1, c2, c3, c4, c5, c6, c7,
         c8, c9, c10, c11, c12, c13, c14, c15,
         c16, c17, c18, c19, c20, c21, c22, c23,
         AA, BB, CC, DD);

  goto INITIALIZE;

 EXIT:
  exit(0);

 E:
 F:
 G:
  goto EXIT;
}
