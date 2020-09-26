#include <stdio.h>
#include <stdlib.h>

#define X_LIMIT 10000000
#define TAPE_LEN ((X_LIMIT * 2) + 10)

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

#define INSTRUCTION(c0, s0, t0, c1, s1, t1,         \
                    c2, s2, t2, c3, s3, t3)         \
  if (TAPE[POS] == 3) ACTION(c3, s3, t3)            \
    else if (TAPE[POS] == 2) ACTION(c2, s2, t2)     \
      else if (TAPE[POS] == 1) ACTION(c1, s1, t1)   \
        else ACTION(c0, s0, t0)

unsigned int XX, AA, BB, CC, DD;
unsigned int PP = 0;

#define RESET_COUNTS XX = AA = BB = CC = DD = 0; PP++;

int c3, c4, c5, c6, c7,
  c8, c9, c10, c11, c12, c13, c14, c15,
  c16, c17, c18, c19, c20, c21, c22, c23;

#define READ(VAR) if ((VAR = getc(stdin)) == EOF) goto EXIT;

#define LOAD_PROGRAM                            \
  READ(c3);                                     \
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
  INSTRUCTION('1', 'R', 'B', c3, c4, c5,
              c6, c7, c8, c9, c10, c11);

 B:
  CHECK_X(BB);
  INSTRUCTION(c12, c13, c14, c15, c16, c17,
              c18, c19, c20, c21, c22, c23);

 H:
  printf("%d | 1RB %c%c%c %c%c%c %c%c%c %c%c%c %c%c%c %c%c%c %c%c%c | %d %d\n",
         PP,
         c3, c4, c5, c6, c7,
         c8, c9, c10, c11, c12, c13, c14, c15,
         c16, c17, c18, c19, c20, c21, c22, c23,
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
