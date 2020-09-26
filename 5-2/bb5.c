#include <stdio.h>

#define PROG "1RB 1LC 1RC 1RB 1RD 0LE 1LA 1LD 1RH 0LA"

#define TAPE_LEN ((100000 * 2) + 10)

int POS = TAPE_LEN / 2;
int TAPE[TAPE_LEN];

#define ZERO_TAPE for (int i = 0; i < TAPE_LEN; i++) {TAPE[i] = 0;}

#define L POS--
#define R POS++

#define INSTRUCTION(c0, s0, t0, c1, s1, t1)     \
  if (TAPE[POS])                                \
    {TAPE[POS] = c1; s1; goto t1;}              \
  else                                          \
    {TAPE[POS] = c0; s0; goto t0;}

int XX, AA, BB, CC, DD, EE;

#define ZERO_COUNTS XX = AA = BB = CC = DD = EE = 0;
#define INCREMENT(COUNT) XX++; COUNT++;

int main (void) {
  ZERO_TAPE;
  ZERO_COUNTS;

 A:
  INCREMENT(AA);
  INSTRUCTION(1, R, B, 1, L, C);

 B:
  INCREMENT(BB);
  INSTRUCTION(1, R, C, 1, R, B);

 C:
  INCREMENT(CC);
  INSTRUCTION(1, R, D, 0, L, E);

 D:
  INCREMENT(DD);
  INSTRUCTION(1, L, A, 1, L, D);

 E:
  INCREMENT(EE);
  INSTRUCTION(1, R, H, 0, L, A);

 H:
  printf("%s | %d | %d %d %d %d %d\n", PROG, XX, AA, BB, CC, DD, EE);
}
