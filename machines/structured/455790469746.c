#define PROGRAM "1RB ...  0LC 0LB  0LD 1LC  1RD 0RE  1LB 1LA"

#define TAPELEN 3000000

#include "machine.h"

int main(void)
{
  // A0
  PRINT;
  RIGHT;

  while (1) {
    while (!BLANK) {
      // B1
      ERASE;
      LEFT;
    }

    // B0
    LEFT;

    while (!BLANK) {
      // C1
      LEFT;
    }

    // C0
    LEFT;

    while (BLANK) {
      // D0
      PRINT;
      RIGHT;
    }

    // D1
    ERASE;
    RIGHT;

    if (BLANK)
      {
        // E0
        PRINT;
        LEFT;
      }
    else
      {
        // E1
        LEFT;

        // A0
        PRINT;
        RIGHT;
      }
  }

 H:
  CHECK_STEPS;
}
