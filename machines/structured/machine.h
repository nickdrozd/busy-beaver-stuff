#include <assert.h>
#include <stdio.h>
#include <stdlib.h>

short TAPE[TAPELEN];
#define CENTER_SQUARE (TAPELEN / 2)

short  POS = CENTER_SQUARE;
short PMIN = CENTER_SQUARE;
short PMAX = CENTER_SQUARE + 1;

#define SQUARE_CHAR(square) square ? '#' : '_'

#define PRINT_TAPE                              \
  for (long i = 0; i < TAPELEN; i++)            \
    {                                           \
      if (i == POS) printf("[");                \
      printf("%c", SQUARE_CHAR(TAPE[i]));       \
      if (i == POS) printf("]");                \
    }                                           \
  printf("\n");

#define CHECK_STEPS assert(STEPS == XLIMIT);

long STEPS = 0;

long MARKS = 0;
#define HALT_IF_BLANK if (!MARKS && STEPS > 0) goto H;

#define SCAN TAPE[POS]

#define WRITE(COLOR) do {                       \
    if (COLOR && !SCAN) MARKS++;                \
    if (!COLOR && SCAN) MARKS--;                \
    TAPE[POS] = COLOR;                          \
  } while (0)

#define BLANK !SCAN
#define PRINT WRITE(1)
#define ERASE WRITE(0)

#define STEP {STEPS++; HALT_IF_BLANK;}

#define SHIFT_EDGE                              \
  if (POS < PMIN) { PMIN--; }                   \
  else if (POS >= PMAX) { PMAX++; }

#define LEFT  { POS--; SHIFT_EDGE; STEP; }
#define RIGHT { POS++; SHIFT_EDGE; STEP; }

#define L -1
#define R 1

#define SHIFT_INTO_EDGE(SH)                     \
  ((POS == PMIN && SH == L) ||                  \
   (POS + 1 == PMAX && SH == R))

#define CHECK_RECUR(SH)                         \
  if (SHIFT_INTO_EDGE(SH)) { goto H; };
