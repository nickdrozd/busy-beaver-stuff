#define TAPELEN 20

#include "machine.h"

int main(void)
{
  while (BLANK) {
    // A0
    PRINT;
    RIGHT;

    // B0
    while (BLANK) {
      PRINT;
      LEFT;
    }

    // B1
    ERASE;
    RIGHT;

    // C0
    while (BLANK) {
      PRINT;
      LEFT;
    }

    // C1
    LEFT;
  }

  // A1
  LEFT;

 H:
  CHECK_STEPS;
}
