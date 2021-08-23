#define TAPELEN 20000

#include "machine.h"

int main(void)
{
 A:
  if (BLANK)
    {
      // A0
      PRINT;
      RIGHT;
      goto B;
    }
  else
    {
      // A1
      PRINT;
      LEFT;
      goto C;
    }

 B:
  while (!BLANK) {
    // B1
    PRINT;
    RIGHT;
  }

  // B0
  PRINT;
  RIGHT;
  goto D;

 C:
  while (!BLANK) {
    // C1
    ERASE;
    RIGHT;
  }

  // C0
  ERASE;
  RIGHT;
  goto D;

 D:
  while (BLANK) {
    // D0
    PRINT;
    LEFT;
  }

  // D1
  PRINT;
  LEFT;
  goto A;

 H:
  PRINT_STEPS;
}
