#include <stdio.h>
#include <stdlib.h>

#define PROG "1RB 1LC 1RC 1RB 1RD 0LE 1LA 1LD 1RH 0LA"

#define X_LIMIT 100000
#define TAPE_LEN ((X_LIMIT * 2) + 10)

int POS = TAPE_LEN / 2;

int TAPE[TAPE_LEN];

#define ZERO_TAPE for (int i = 0; i < TAPE_LEN; i++) {TAPE[i] = 0;}

#define L POS--
#define R POS++

#define INSTRUCTION(c0, s0, t0, c1, s1, t1) \
  if (TAPE[POS]) {TAPE[POS] = c1; s1; goto t1;} else {TAPE[POS] = c0; s0; goto t0;}

int x_count = 0;

int main (void) {
  ZERO_TAPE;

 A:
  x_count++;

  INSTRUCTION(1, R, B, 1, L, C);

 B:
  x_count++;

  INSTRUCTION(1, R, C, 1, R, B);

 C:
  x_count++;

  INSTRUCTION(1, R, D, 0, L, E);

 D:
  x_count++;

  INSTRUCTION(1, L, A, 1, L, D);

 E:
  x_count++;

  INSTRUCTION(1, R, H, 0, L, A);

 H:
  printf("%s | %d\n", PROG, x_count);
  exit(0);
}
