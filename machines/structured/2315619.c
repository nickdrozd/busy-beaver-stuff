#define PROGRAM "1RB 2LB 1LC  1LA 2RB 1RB  1R_ 2LA 0LC"

#define TAPELEN 48

#include "machine.h"

int main(void)
{
  while (1) {
    if (SCAN == 2) {
      // A2
      WRITE(1);
      LEFT;

      while (1) {
        if (SCAN == 2) {
          // C2
          WRITE(0);
          LEFT;
          continue;
        }

        if (SCAN == 0) {
          // C0
          WRITE(1);
          RIGHT;
          goto H;
        }

        if (SCAN == 1) {
          // C1
          WRITE(2);
          LEFT;
          break;
        }
      }
    }

    else {
      if (BLANK) {
        // A0
        WRITE(1);
        RIGHT;
      } else {
        // A1
        WRITE(2);
        LEFT;
      }

      while (!BLANK) {
        if (SCAN == 1)
          // B1
          WRITE(2);
        else
          // B2
          WRITE(1);

        RIGHT;
      }

      // B0
      WRITE(1);
      LEFT;
    }
  }

 H:
  CHECK_STEPS;
}
