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

 C:
  switch (SCAN) {
    case 0:
      // C0
      WRITE(3);
      RIGHT;
      goto D;
    case 1:
      // C1
      WRITE(3);
      LEFT;
      goto C;
    case 3:
      // C3
      WRITE(1);
      LEFT;
      goto C;
    case 4:
      // C4
      WRITE(1);
      RIGHT;
      goto H;
  }

 D:
  switch (SCAN) {
    case 1:
      // D1
      WRITE(1);
      RIGHT;
      goto D;
    case 2:
      // D2
      WRITE(1);
      RIGHT;
      // B0
      WRITE(2);
      LEFT;
      goto C;
    case 3:
      // D3
      WRITE(1);
      LEFT;
      goto E;
  }

 E:
  switch (SCAN) {
    case 0:
      // E0
      WRITE(4);
      RIGHT;
      goto D;
    case 1:
      // E1
      WRITE(1);
      LEFT;
      goto E;
    case 3:
      // E3
      WRITE(1);
      RIGHT;
      goto D;
    case 4:
      // E4
      WRITE(1);
      LEFT;
      goto C;
  }

 H:
  CHECK_STEPS;
}
