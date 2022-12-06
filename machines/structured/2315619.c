#define PROGRAM "1RB 2LB 1LC  1LA 2RB 1RB  1R_ 2LA 0LC"

#define TAPELEN 48

#include "machine.h"

int main(void)
{
 A:
  switch (SCAN) {
    case 0:
      // A0
      WRITE(1);
      RIGHT;
      goto B;

    case 1:
      // A1
      WRITE(2);
      LEFT;
      goto B;

    case 2:
      // A2
      WRITE(1);
      LEFT;
      goto C;
  }

 B:
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
  goto A;

 C:
  switch (SCAN) {
    case 0:
      // C0
      WRITE(1);
      RIGHT;
      goto H;

    case 1:
      // C1
      WRITE(2);
      LEFT;
      goto A;

    case 2:
      // C2
      WRITE(0);
      LEFT;
      goto C;
  }

 H:
  CHECK_STEPS;
}
