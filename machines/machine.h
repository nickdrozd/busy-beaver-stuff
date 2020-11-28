#include <stdio.h>
#include <stdlib.h>

#define TAPE_LEN (X_LIMIT * 2)

#define SETUP_TAPE                              \
  unsigned int POS;                             \
  unsigned int PMIN;                            \
  unsigned int PMAX;                            \
  unsigned int TAPE[TAPE_LEN];                  \
  unsigned int i;

#define DISPATCH_TABLE                          \
  static void* dispatch[] =                     \
    { &&A, &&B, &&C, &&D, &&E, &&F, &&G, &&H };

#define IN_RANGE(COUNT) (LOWER_BOUND <= COUNT && COUNT < UPPER_BOUND)

#define CHECK_X(COUNT) {                        \
    XX++;                                       \
    if (XX > X_LIMIT) {goto H;};                \
    COUNT = XX;                                 \
  }

#define RESET_TAPE                              \
  for (i = PMIN; i < PMAX; i++) {               \
    TAPE[i] = 0;                                \
  }                                             \
  POS = TAPE_LEN / 2;                           \
  PMIN = POS;                                   \
  PMAX = POS + 1;

#define SCAN(COLOR) TAPE[POS] == COLOR

#define ACTION(c, s, t) {                       \
    TAPE[POS] = c;                              \
    POS += s;                                   \
    if (POS < PMIN) { PMIN--; }                 \
    else if (POS >= PMAX) { PMAX++; }           \
    goto *dispatch[t];                          \
  }

#define INSTRUCTION(c0, s0, t0, c1, s1, t1)     \
  if (SCAN(1))                                  \
    ACTION(c1, s1, t1)                          \
    else                                        \
      ACTION(c0, s0, t0)

#define NEXT getc(stdin)

#define COLOR_CONV '0'
#define SHIFT_CONV 'L'
#define TRANS_CONV 'A'

#define L -1
#define R 1

#define READ_BOUND    if (NEXT == EOF) goto EXIT;
#define READ_COLOR(C) C = NEXT - COLOR_CONV;
#define READ_SHIFT(S) S = NEXT == SHIFT_CONV ? L : R;
#define READ_TRANS(T) T = NEXT - TRANS_CONV;

#define READ_ACTION(C, S, T) {                  \
    READ_COLOR(C);                              \
    READ_SHIFT(S);                              \
    READ_TRANS(T);                              \
  }

#define FORMAT_INSTR(C, S, T)                       \
  C + COLOR_CONV, S == R ? 'R' : 'L', T + TRANS_CONV

#define A0C '1' - COLOR_CONV
#define A0S 'R' - SHIFT_CONV - 5;
#define A0T 'B' - TRANS_CONV
