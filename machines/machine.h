#include <stdio.h>
#include <stdlib.h>

#define SETUP                                   \
  SETUP_TAPE                                    \
  SETUP_COUNTS

#define TAPE_LEN (X_LIMIT * 2)

#define CENTER_SQUARE (TAPE_LEN / 2)

#define SETUP_TAPE                              \
  unsigned int POS = CENTER_SQUARE;             \
  unsigned int PMIN = CENTER_SQUARE;            \
  unsigned int PMAX = CENTER_SQUARE + 1;        \
  unsigned int TAPE[TAPE_LEN] = { 0 };          \
  unsigned int i;

#define DISPATCH_TABLE                          \
  static void* dispatch[] =                     \
    { &&A, &&B, &&C, &&D, &&E, &&F, &&G, &&H };

#define SETUP_COUNTS                            \
  unsigned int MARKS = 0;                       \
  unsigned int COUNTS[STATES];                  \
  unsigned int XX;                              \
  unsigned int PP = 0;

#define RESET_COUNTS                            \
  XX = MARKS = 0;                               \
  PP++;                                         \
  for (i = 0; i < STATES; i++) {                \
    COUNTS[i] = 0;                              \
  }

#define AA COUNTS[0]
#define BB COUNTS[1]
#define CC COUNTS[2]
#define DD COUNTS[3]
#define EE COUNTS[4]

#define IN_RANGE(COUNT) (LOWER_BOUND <= COUNT && COUNT < UPPER_BOUND)

#define CHECK_X(COUNT) {                        \
    if (++XX > X_LIMIT) {goto H;};              \
    COUNT = XX;                                 \
  }

#define WIPE_AND_SCORE                          \
  for (i = PMIN; i < PMAX; i++) {               \
    TAPE[i] = 0;                                \
  }                                             \
  POS = CENTER_SQUARE;                          \
  PMIN = CENTER_SQUARE;                         \
  PMAX = CENTER_SQUARE + 1;

#define HALT_IF_BLANK                           \
  if (!MARKS) { goto H; };

#define SCAN(COLOR) TAPE[POS] == COLOR

#define DO_COLOR(c)                             \
  if (c && !TAPE[POS]) { MARKS++; }             \
  else if (!c && TAPE[POS]) { MARKS--; }        \
  TAPE[POS] = c;

#define DO_SHIFT(s)                             \
  POS += s;                                     \
  if (POS < PMIN) { PMIN--; }                   \
  else if (POS >= PMAX) { PMAX++; }

#define DO_TRANS(t)                             \
  goto *dispatch[t];

#define ACTION(c, s, t) {                       \
    DO_COLOR(c);                                \
    HALT_IF_BLANK;                              \
    DO_SHIFT(s);                                \
    DO_TRANS(t);                                \
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
