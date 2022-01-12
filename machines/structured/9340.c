#define PROGRAM "2LA 3LB 0RA 0RB  1RA 2RB 1RB 2RA"

#define TAPELEN 300

#include "machine.h"

int main(void)
{
  WRITE(1);
  RIGHT;

  RIGHT;

  WRITE(1);
  RIGHT;

  WRITE(1);
  RIGHT;

  WRITE(1);
  RIGHT;

  while (1) {
    while (SCAN == 0 || SCAN == 2) {
      if (SCAN == 0) {
        // A0
        WRITE(2);
        LEFT;
      } else if (SCAN == 2) {
        // A2
        WRITE(0);
        RIGHT;
      }
    }

    if (SCAN == 1) {
      // A1
      WRITE(3);
      LEFT;
    } else if (SCAN == 3) {
      // A3
      WRITE(0);
      RIGHT;
    }

    while (SCAN == 1 || SCAN == 2) {
      if (SCAN == 1)
        // B1
        WRITE(2);
      else if (SCAN == 2)
        // B2
        WRITE(1);

      RIGHT;
    }

    if (SCAN == 0)
      // B0
      WRITE(1);
    else if (SCAN == 3)
      // B3
      WRITE(2);

    RIGHT;
  }

 H:
  CHECK_STEPS;
}
