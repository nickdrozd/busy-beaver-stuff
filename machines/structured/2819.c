#define PROGRAM "1RB 1RC  1LC 1RD  1RA 1LD  0RD 0LB"

#define TAPELEN 300

#include "machine.h"

int main(void)
{
  // A0
  PRINT;
  RIGHT;

  // B0
  PRINT;
  LEFT;

  while (1) {
    if (BLANK)
      {
        // C0
        PRINT;
        RIGHT;

        if (!BLANK) {
          // A1
          RIGHT;
          continue;
        }

        // A0
        PRINT;
        RIGHT;
      }
    else
      {
        // C1
        LEFT;

        while (BLANK) {
          // D0
          CHECK_RECUR(R);
          RIGHT;
        }

        // D1
        ERASE;
        LEFT;
      }

    while (!BLANK) {
      // B1
      RIGHT;

      while (BLANK) {
        // D0
        CHECK_RECUR(R);
        RIGHT;
      }

      // D1
      ERASE;
      LEFT;
    }

    // B0
    PRINT;
    LEFT;
  }

 H:
  CHECK_STEPS;
}
