#define PROGRAM "1RB 2LA 1RA 1RA  1LB 1LA 3RB 1RH"

#define TAPELEN 2080

#include "machine.h"

int main(void)
{
  // A0
  WRITE(1);
  RIGHT;

  while (1) {
    switch (SCAN) {
      case 0:
        // B0
        WRITE(1);
        LEFT;
        break;

      case 2:
        // B2
        WRITE(3);
        RIGHT;
        break;

      case 3:
        // B3
        WRITE(1);
        RIGHT;
        goto H;

      case 1:
        // B1
        WRITE(1);
        LEFT;

        while (!BLANK) {
          switch (SCAN) {
            case 1:
              // A1
              WRITE(2);
              LEFT;
              break;
            case 2:
              // A2
              WRITE(1);
              RIGHT;
              break;
            case 3:
              // A3
              WRITE(1);
              RIGHT;
              break;
          }
        }

        // A0
        WRITE(1);
        RIGHT;
        break;
    }
  }

 H:
  CHECK_STEPS;
}
