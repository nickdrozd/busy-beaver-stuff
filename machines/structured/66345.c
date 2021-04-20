#define TAPELEN 1000

#include "machine.h"

int main(void)
{
  while (1) {
    if (BLANK)
      {
        // A0
        PRINT;
        RIGHT;

        if (!BLANK)
          {
            // B1
            ERASE;
            LEFT;
          }
        else
          {
            // B0
            PRINT;
            LEFT;

            // D1
            while (!BLANK) {
              ERASE;
              LEFT;
            }

            // D0
            PRINT;
            LEFT;
          }
      }
    else
      {
        // A1
        ERASE;
        LEFT;

        // C0
        while (BLANK) {
          PRINT;
          RIGHT;
        }

        // C1
        PRINT;
        RIGHT;

        // D1
        while (!BLANK) {
          ERASE;
          LEFT;
        }

        // D0
        PRINT;
        LEFT;
      }
  }

 H:
  PRINT_STEPS;
}
