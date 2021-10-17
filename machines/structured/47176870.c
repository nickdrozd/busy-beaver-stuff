#define TAPELEN 24408

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

 C:
  if (BLANK)
    {
      // C0
      PRINT;
      RIGHT;
      goto D;
    }
  else
    {
      // C1
      ERASE;
      LEFT;
      goto E;
    }

 D:
  while (!BLANK) {
    // D1
    PRINT;
    LEFT;
  }

  // D0
  PRINT;
  LEFT;
  goto A;

 E:
  if (BLANK)
    {
      // E0
      PRINT;
      RIGHT;
      goto H;
    }
  else
    {
      // E1
      ERASE;
      LEFT;
      goto A;
    }

 H:
  CHECK_STEPS;
}
