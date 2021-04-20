#define TAPELEN 1000

#include "machine.h"

int main(void)
{
 A:
  if (BLANK)
    {
      // A0
      PRINT;
      RIGHT;

      if (BLANK)
        {
          // B0
          PRINT;
          LEFT;
          goto D;
        }
      else
        {
          // B1
          ERASE;
          LEFT;
          goto A;
        }
    }
  else
    {
      // A1
      ERASE;
      LEFT;

      // C0
      while (BLANK) {
        PRINT;
        RIGHT;
      }

      // C1
      PRINT;
      RIGHT;
      goto D;
    }

 D:
  // D1
  while (!BLANK) {
    ERASE;
    LEFT;
  }

  // D0
  PRINT;
  LEFT;
  goto A;

 H:
  PRINT_STEPS;
}
