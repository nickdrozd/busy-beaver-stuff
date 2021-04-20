#define TAPELEN 20

#include "machine.h"

int main(void)
{
 A:
  if (BLANK)
    {
      PRINT;
      RIGHT;
      goto B;
    }
  else
    {
      PRINT;
      LEFT;
      goto H;
    }

 B:
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

 H:
  PRINT_TAPE;
  PRINT_STEPS;
}
