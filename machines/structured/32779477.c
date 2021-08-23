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
  if (BLANK)
    {
      // B0
      PRINT;
      RIGHT;
      goto D;
    }
  else
    {
      // B1
      PRINT;
      RIGHT;
      goto B;
    }

 C:
  if (BLANK)
    {
      // C0
      ERASE;
      RIGHT;
      goto D;
    }
  else
    {
      // C1
      ERASE;
      RIGHT;
      goto C;
    }

 D:
  if (BLANK)
    {
      // D0
      PRINT;
      LEFT;
      goto D;
    }
  else
    {
      // D1
      PRINT;
      LEFT;
      goto A;
    }

 H:
  PRINT_STEPS;
}
