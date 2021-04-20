#define TAPELEN 20

#include "machine.h"

int main(void)
{
 A:
  if (!BLANK)
    {
      // A1
      PRINT;
      LEFT;
      goto H;
    }
  else
    {
      // A0
      PRINT;
      RIGHT;

      // B0
      while (BLANK) {
        PRINT;
        LEFT;
      }

      // B1
      ERASE;
      RIGHT;

      // C0
      while (BLANK) {
        PRINT;
        LEFT;
      }

      // C1
      PRINT;
      LEFT;
      goto A;
    }

 H:
  PRINT_TAPE;
  PRINT_STEPS;
}
