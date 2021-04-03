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

#define TCOB CHECK_LIMIT; //PRINT_TAPE;

#define SCAN(COLOR) (TAPE[POS] == COLOR)
#define WRITE(COLOR) TAPE[POS] = COLOR;

#define BLANK !TAPE[POS]
#define PRINT WRITE(1)
#define ERASE WRITE(0)

#define LEFT  {POS--; STEPS++;}
#define RIGHT {POS++; STEPS++;}
