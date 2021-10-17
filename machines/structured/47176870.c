#define TAPELEN 24408

#include "machine.h"

int main(void)
{
  while (1) {
    if (!BLANK)
      {
        // A1
        LEFT;
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
        RIGHT;
      }

    if (BLANK)
      {
        // C0
        PRINT;
        RIGHT;

        while (!BLANK) {
          // D1
          LEFT;
        }

        // D0
        PRINT;
        LEFT;
      }
    else
      {
        // C1
        ERASE;
        LEFT;

        if (BLANK) {
            // E0
            PRINT;
            RIGHT;
            break;
        }

        // E1
        ERASE;
        LEFT;
      }
  }

 H:
  CHECK_STEPS;
}
