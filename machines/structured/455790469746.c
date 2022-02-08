#define PROGRAM "1RB ...  0LC 0LB  0LD 1LC  1RD 0RE  1LB 1LA"

#define TAPELEN 3000000

#include "machine.h"

int main(void)
{
 A:
  // A0
  PRINT;
  RIGHT;
  goto B;

 B:
  if (BLANK)
    {
      // B0
      LEFT;
      goto C;
    }
  else
    {
      // B1
      ERASE;
      LEFT;
      goto B;
    }

 C:
  if (BLANK)
    {
      // C0
      LEFT;
      goto D;
    }
  else
    {
      // C1

      LEFT;
      goto C;
    }

 D:
  if (BLANK)
    {
      // D0
      PRINT;
      RIGHT;
      goto D;
    }
  else
    {
      // D1
      ERASE;
      RIGHT;
      goto E;
    }

 E:
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
