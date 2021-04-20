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
      ERASE;
      LEFT;
      goto C;
    }

 B:
  if (BLANK)
    {
      PRINT;
      LEFT;
      goto D;
    }
  else
    {
      ERASE;
      LEFT;
      goto A;
    }

 C:
  if (BLANK)
    {
      PRINT;
      RIGHT;
      goto C;
    }
  else
    {
      PRINT;
      RIGHT;
      goto D;
    }

 D:
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
      goto D;
    }

 H:
  PRINT_STEPS;
}
