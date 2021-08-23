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
          PRINT;
          RIGHT;
        }

        // B0
        PRINT;
        RIGHT;
      }
    else
      {
        // A1
        PRINT;
        LEFT;

        while (!BLANK) {
          // C1
          ERASE;
          RIGHT;
        }

        // C0
        ERASE;
        RIGHT;
      }

    while (BLANK) {
      // D0
      PRINT;
      LEFT;
    }

    // D1
    PRINT;
    LEFT;
  }

 H:
  PRINT_STEPS;
}
