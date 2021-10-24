#define TAPELEN 20000

#include "machine.h"

int main(void)
{
  // (A0)
  PRINT;
  RIGHT;

  // (B0)
  PRINT;
  RIGHT;

  while (1) {
    while (BLANK) {
      // D0
      PRINT;
      LEFT;
    }

    // D1
    LEFT;

    if (!BLANK)
      {
        // A1
        LEFT;

        while (!BLANK) {
          // C1
          ERASE;
          RIGHT;
        }
      }
    else
      {
        // A0
        PRINT;

        do {
          // B1
          RIGHT;
        } while (!BLANK);

        // B0
        PRINT;
      }

    // B0, C0
    RIGHT;
  }

 H:
  CHECK_STEPS;
}
