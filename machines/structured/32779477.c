#define TAPELEN 20000

#include "machine.h"

int main(void)
{
  while (1) {
    if (BLANK)
      {
        // A0
        PRINT;
        RIGHT;

        while (!BLANK) {
          // B1
          RIGHT;
        }

        // B0
        PRINT;
      }
    else
      {
        // A1
        LEFT;

        while (!BLANK) {
          // C1
          ERASE;
          RIGHT;
        }
      }

    // B0, C0
    RIGHT;

    while (BLANK) {
      // D0
      PRINT;
      LEFT;
    }

    // D1
    LEFT;
  }

 H:
  PRINT_STEPS;
}
