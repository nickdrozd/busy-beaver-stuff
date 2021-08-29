#include <stdio.h>
#include <stdlib.h>

short TAPE[TAPELEN];
short POS = TAPELEN / 2;

short STORE;

#define SQUARE_CHAR(square) square ? '#' : '_'

#define PRINT_TAPE                              \
  for (long i = 0; i < TAPELEN; i++)            \
    {                                           \
      if (i == POS) printf("[");                \
      printf("%c", SQUARE_CHAR(TAPE[i]));       \
      if (i == POS) printf("]");                \
    }                                           \
  printf("\n");

#define PRINT_STEPS printf("%ld\n", STEPS);

long STEPS = 0;
#define CHECK_LIMIT if (STEPS > XLIMIT) return 0;

long MARKS = 0;
#define HALT_IF_BLANK if (!MARKS && STEPS > 0) goto H;

#define SCAN(COLOR) (TAPE[POS] == COLOR)
#define WRITE(COLOR) do {                       \
    if (COLOR && !TAPE[POS]) MARKS++;           \
    if (!COLOR && TAPE[POS]) MARKS--;           \
    TAPE[POS] = COLOR;                          \
  } while (0)

#define BLANK !TAPE[POS]
#define PRINT WRITE(1)
#define ERASE WRITE(0)

#define STEP {STEPS++; HALT_IF_BLANK;}

#define LEFT  {POS--; STEP;}
#define RIGHT {POS++; STEP;}
