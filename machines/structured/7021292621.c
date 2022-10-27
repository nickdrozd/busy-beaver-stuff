#define PROGRAM "1RB 4LA 1LA 1R_ 2RB  2LB 3LA 1LB 2RA 0RB"

#define TAPELEN 110

#include "machine.h"

int main(void)
{
  return 0;  // takes too long
 A:
  switch (SCAN) {
    case 0:
      // A0
      WRITE(1);
      RIGHT;
      goto B;
    case 1:
      // A1
      WRITE(4);
      LEFT;
      goto A;
    case 2:
      // A2
      WRITE(1);
      LEFT;
      goto A;
    case 3:
      // A3
      WRITE(1);
      RIGHT;
      goto H;
    case 4:
      // A4
      WRITE(2);
      RIGHT;
      goto B;
  }

 B:
  switch (SCAN) {
    case 0:
      // B0
      WRITE(2);
      LEFT;
      goto B;
    case 1:
      // B1
      WRITE(3);
      LEFT;
      goto A;
    case 2:
      // B2
      WRITE(1);
      LEFT;
      goto B;
    case 3:
      // B3
      WRITE(2);
      RIGHT;
      goto A;
    case 4:
      // B4
      WRITE(0);
      RIGHT;
      goto B;
  }

 H:
  CHECK_STEPS;
}
