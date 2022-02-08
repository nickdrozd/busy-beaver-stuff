#define PROGRAM "1RB ...  0LC 0LB  0LD 1LC  1RD 0RE  1LB 1LA"

#define TAPELEN 3000000

#include "machine.h"

int main(void)
{
 A:
  // A0
  PRINT;
  RIGHT;

 B:
  while (!BLANK) {
    // B1
    ERASE;
    LEFT;
  }

  // B0
  LEFT;

  while (!BLANK) {
    // C1
    LEFT;
  }

  // C0
  LEFT;

  while (BLANK) {
    // D0
    PRINT;
    RIGHT;
  }

  // D1
  ERASE;
  RIGHT;

  if (BLANK)
    {
      // E0
      PRINT;
      LEFT;
      goto B;
    }
  else
    {
      // E1
      LEFT;
      goto A;
    }

 H:
  CHECK_STEPS;
}
