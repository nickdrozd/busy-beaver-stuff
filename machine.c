#include <stdio.h>
#include <stdlib.h>

#define PROG "1RB 1RC 1LC 1RD 1RA 1LD 0RD 0LB"

#define X_LIMIT 100000
#define TAPE_LEN ((X_LIMIT * 2) + 10)

#define CHECK_X do {x_count++; if (x_count > X_LIMIT) {goto H;}} while (0)

int POS = TAPE_LEN / 2;

int TAPE[TAPE_LEN];

#define ZERO_TAPE for (int i = 0; i < TAPE_LEN; i++) {TAPE[i] = 0;}

#define L POS--
#define R POS++

#define INSTRUCTION(c0, s0, t0, c1, s1, t1)     \
  if (TAPE[POS]) {TAPE[POS] = c1; s1; goto t1;} \
  else {TAPE[POS] = c0; s0; goto t0;}

int x_count = 0;

int a_count = 0;
int b_count = 0;
int c_count = 0;
int d_count = 0;

char c0, c1, c2, c3, c4, c5, c6, c7,
  c8, c9, c10, c11, c12, c13, c14, c15,
  c16, c17, c18, c19, c20, c21, c22, c23;

#define READ(var) var = getc(stdin);

#define LOAD_PROGRAM                            \
  READ(c0); READ(c1); READ(c2); READ(c3);       \
  READ(c4); READ(c5); READ(c6); READ(c7);       \
  READ(c8); READ(c9); READ(c10); READ(c11);     \
  READ(c12); READ(c13); READ(c14); READ(c15);   \
  READ(c16); READ(c17); READ(c18); READ(c19);   \
  READ(c20); READ(c21); READ(c22); READ(c23);   \
  getc(stdin);

int main (void) {
  void* dispatch[] { &&A, &&B, &&C, &&D };

 INITIALIZE:
  ZERO_TAPE;
  LOAD_PROGRAM;

 A:
  CHECK_X; a_count = x_count;

  INSTRUCTION(1, R, B, 1, R, C);

 B:
  CHECK_X; b_count = x_count;

  INSTRUCTION(1, L, C, 1, R, D);

 C:
  CHECK_X; c_count = x_count;

  INSTRUCTION(1, R, A, 1, L, D);

 D:
  CHECK_X; d_count = x_count;

  INSTRUCTION(0, R, D, 0, L, B);

 H:
  printf("%s | %d %d %d %d\n", PROG, a_count, b_count, c_count, d_count);
  exit(0);
}
