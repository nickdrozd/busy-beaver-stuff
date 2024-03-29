#define PROGRAM "1RB ... ... ... ...  2LC ... ... ... ...  3RD 3LC ... 1LC 1R_  ... 1RD 1RB 1LE ...  4RD 1LE ... 1RD 1LC"

#define TAPELEN 10000

#include "machine.h"

int main(void)
{
  // A0
  WRITE(1);
  RIGHT;

  // B0
  WRITE(2);
  LEFT;

  // C1
  WRITE(3);
  LEFT;

  // C0
  WRITE(3);
  RIGHT;

  while (1) {
    while (SCAN == 1) {
      // D1
      WRITE(1);
      RIGHT;
    }

    if (SCAN == 2) {
      // D2
      WRITE(1);
      RIGHT;

      // B0
      WRITE(2);
      LEFT;

      while (1) {
        if (SCAN == 0)
          break;

        if (SCAN == 4) {
          // C4
          WRITE(1);
          RIGHT;
          goto H;
        }

        if (SCAN == 1)
          // C1
          WRITE(3);
        else if (SCAN == 3)
          // C3
          WRITE(1);

        LEFT;
      }

      // C0
      WRITE(3);
      RIGHT;

      continue;
    }

    // D3
    WRITE(1);
    LEFT;

    while (SCAN == 1) {
      // E1
      WRITE(1);
      LEFT;
    }

    if (SCAN == 4) {
      // E4
      WRITE(1);
      LEFT;

      while (1) {
        if (SCAN == 0)
          break;

        if (SCAN == 4) {
          // C4
          WRITE(1);
          RIGHT;
          goto H;
        }

        if (SCAN == 1)
          // C1
          WRITE(3);
        else if (SCAN == 3)
          // C3
          WRITE(1);

        LEFT;
      }

      // C0
      WRITE(3);
      RIGHT;

      continue;
    }

    if (SCAN == 0)
      // E0
      WRITE(4);
    else
      // E3
      WRITE(1);

    RIGHT;
  }

 H:
  CHECK_STEPS;
}
