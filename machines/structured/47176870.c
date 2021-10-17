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
  if (BLANK)
    {
      // B0
      PRINT;
      RIGHT;
      goto C;
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
  if (BLANK)
    {
      // D0
      PRINT;
      LEFT;
      goto A;
    }
  else
    {
      // D1
      PRINT;
      LEFT;
      goto D;
    }

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
