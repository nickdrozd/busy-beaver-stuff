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
  goto C;

 C:
  if (BLANK)
    {
      PRINT;
      LEFT;
      goto C;
    }
  else
    {
      PRINT;
      LEFT;
      goto A;
    }

 H:
  PRINT_TAPE;
  PRINT_STEPS;
}
