#include <stdio.h>
#include <stdlib.h>

#define PROG "1RB 1RC 1LC 1RD 1RA 1LD 0RD 0LB"

#define X_LIMIT 100000
#define TAPE_LEN ((X_LIMIT * 2) + 10)

#define CHECK_X(COUNT) do {                     \
    x_count++;                                  \
    if (x_count > X_LIMIT) {goto H;};           \
    COUNT = x_count;                            \
  } while (0)

int POS = TAPE_LEN / 2;

int TAPE[TAPE_LEN];

#define ZERO_TAPE for (int i = 0; i < TAPE_LEN; i++) { TAPE[i] = 0; }

#define L POS--;
#define R POS++;

#define ACTION(c, s, t) {                       \
    TAPE[POS] = c;                              \
    if (s - 76) { R } else { L };               \
    goto *dispatch[t - 65];                     \
  }

#define INSTRUCTION(c0, s0, t0, c1, s1, t1)     \
  if (TAPE[POS])                                \
    ACTION(c1, s1, t1)                          \
    else                                        \
      ACTION(c0, s0, t0)

int x_count = 0;

int a_count = 0;
int b_count = 0;
int c_count = 0;
int d_count = 0;

#define RESET_COUNTS a_count = b_count = c_count = d_count = 0;

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
  static void* dispatch[] = { &&A, &&B, &&C, &&D };

 INITIALIZE:
  RESET_COUNTS;
  ZERO_TAPE;
  LOAD_PROGRAM;

  printf("%d\n", c1 - 76);

 A:
  CHECK_X(a_count);
  INSTRUCTION(c0, c1, c2, c3, c4, c5);

 B:
  CHECK_X(b_count);
  INSTRUCTION(c6, c7, c8, c9, c10, c11);

 C:
  CHECK_X(c_count);

  INSTRUCTION(c12, c13, c14, c15, c16, c17);

 D:
  CHECK_X(d_count);

  INSTRUCTION(c18, c19, c20, c21, c22, c23);

 H:
  printf("%c%c%c %c%c%c %c%c%c %c%c%c %c%c%c %c%c%c %c%c%c %c%c%c | %d %d %d %d\n",
         c0, c1, c2, c3, c4, c5, c6, c7,
         c8, c9, c10, c11, c12, c13, c14, c15,
         c16, c17, c18, c19, c20, c21, c22, c23,
         a_count, b_count, c_count, d_count);

  goto INITIALIZE;

 EXIT:
  exit(0);
}
