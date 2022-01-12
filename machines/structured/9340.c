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

 A:
  switch (SCAN) {
    case 0:
      // A0
      WRITE(2);
      LEFT;
      goto A;
    case 1:
      // A1
      WRITE(3);
      LEFT;
      goto B;
    case 2:
      // A2
      WRITE(0);
      RIGHT;
      goto A;
    case 3:
      // A3
      WRITE(0);
      RIGHT;
      goto B;
  }

 B:
  switch (SCAN) {
    case 0:
      // B0
      WRITE(1);
      RIGHT;
      goto A;
    case 1:
      // B1
      WRITE(2);
      RIGHT;
      goto B;
    case 2:
      // B2
      WRITE(1);
      RIGHT;
      goto B;
    case 3:
      // B3
      WRITE(2);
      RIGHT;
      goto A;
  }

 H:
  CHECK_STEPS;
}
