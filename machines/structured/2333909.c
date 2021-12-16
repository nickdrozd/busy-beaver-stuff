#define PROGRAM "1RB 2RB 1LB 1LA  1LB 3RA 3LA 2RB"

#define TAPELEN 4065

#include "machine.h"

int main(void)
{
  // A0
  WRITE(1);
  RIGHT;

 B:
  switch (SCAN) {
    case 0:
      // B0
      CHECK_RECUR(L);
      WRITE(1);
      LEFT;
      goto B;
    case 1:
      // B1
      WRITE(3);
      RIGHT;
      goto A;
    case 2:
      // B2
      WRITE(3);
      LEFT;
      goto A;
    case 3:
      // B3
      WRITE(2);
      RIGHT;
      goto B;
  }

 A:
  while (SCAN == 3) {
    // A3
    WRITE(1);
    LEFT;
  }

  switch (SCAN) {
    case 0:
      // A0
      WRITE(1);
      RIGHT;
      goto B;
    case 1:
      // A1
      WRITE(2);
      RIGHT;
      goto B;
    case 2:
      // A2
      WRITE(1);
      LEFT;
      goto B;
  }

 H:
  CHECK_STEPS;
}
