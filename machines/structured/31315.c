#define TAPELEN 1000

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
      LEFT;
      goto C;
    }

 B:
  if (BLANK)
    {
      LEFT;
      goto C;
    }
  else
    {
      ERASE;
      RIGHT;
      goto D;
    }

 C:
  if (!BLANK)
    {
      LEFT;
      goto E;
    }
  else
    {
      PRINT;
      RIGHT;
      goto D;
    }

 D:
  if (!BLANK)
    {
      LEFT;
      goto A;
    }
  else
    {
      PRINT;
      RIGHT;
      goto E;
    }

 E:
  if (BLANK)
    {
      PRINT;
      LEFT;
      goto A;
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
