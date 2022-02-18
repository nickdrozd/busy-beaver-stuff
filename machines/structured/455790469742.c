#define PROGRAM "1RB ...  0RC 1RB  1LC 1LD  1RE 0RD  0LE 0RB"

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
      RIGHT;
    }

    // B0
    RIGHT;

    while (BLANK) {
      // C0
      PRINT;
      LEFT;
    }

    // C1
    LEFT;

    while (!BLANK)  {
      // D1
      ERASE;
      RIGHT;
    }

    // D0
    PRINT;
    RIGHT;

    while (BLANK) {
      // E0
      LEFT;
    }

    // E1
    ERASE;
    RIGHT;
  }

 H:
  CHECK_STEPS;
}
