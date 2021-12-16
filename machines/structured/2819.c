#define PROGRAM "1RB 1RC  1LC 1RD  1RA 1LD  0RD 0LB"

#define TAPELEN 300

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
      RIGHT;
      goto C;
    }

 B:
  if (BLANK)
    {
      PRINT;
      LEFT;
      goto C;
    }
  else
    {
      RIGHT;
      goto D;
    }

 C:
  if (BLANK)
    {
      PRINT;
      RIGHT;
      goto A;
    }
  else
    {
      LEFT;
      goto D;
    }

 D:
  CHECK_RECUR(R);
  if (BLANK)
    {
      RIGHT;
      goto D;
    }
  else
    {
      ERASE;
      LEFT;
      goto B;
    }

 H:
  CHECK_STEPS;
}
