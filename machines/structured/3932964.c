#define PROGRAM "1RB 2LA 1RA 1RA  1LB 1LA 3RB 1RH"

#define TAPELEN 2080

#include "machine.h"

int main(void)
{
 A:
  switch (SCAN) {
    case 0:
      WRITE(1);
      RIGHT;
      goto B;
    case 1:
      WRITE(2);
      LEFT;
      goto A;
    case 2:
      WRITE(1);
      RIGHT;
      goto A;
    case 3:
      WRITE(1);
      RIGHT;
      goto A;
  }

 B:
  switch (SCAN) {
    case 0:
      WRITE(1);
      LEFT;
      goto B;
    case 1:
      WRITE(1);
      LEFT;
      goto A;
    case 2:
      WRITE(3);
      RIGHT;
      goto B;
    case 3:
      WRITE(1);
      RIGHT;
      goto H;
  }

 H:
  CHECK_STEPS;
}
