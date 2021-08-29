#define TAPELEN 20

#include "machine.h"

int main(void)
{
  while (1) {
    if (BLANK)
      {
        // A0
        PRINT;
        RIGHT;
      }
    else
      {
        // A1
        LEFT;
      }

    if (BLANK)
      {
        // B0
        PRINT;
        LEFT;
      }
    else
      {
        // B1
        LEFT;
        break;
      }
  }

  while (1) {
    if (BLANK)
      {
        // C0
        PRINT;
        RIGHT;
      }
    else
      {
        // C1
        ERASE;
        LEFT;
      }
  }

 H:
  CHECK_STEPS;
}
