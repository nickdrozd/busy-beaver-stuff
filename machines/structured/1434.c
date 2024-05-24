#define PROGRAM "1RB ... ... ...  0LC 2LC ... ...  0LC 3RD 0RD 2RE  1LF 1LC 1RB ...  ... 3RD ... ...  1LG ... 2RB 1LF  2RE ... 2LC ..."

#define TAPELEN 1000

#include "machine.h"

int main(void)
{
  // A0
  WRITE(1);
  RIGHT;

 B:
  // B1
  if (SCAN == 1) {
    WRITE(2);
  }
  // B0 / B1
  LEFT;
  goto C;

 C:
  switch (SCAN) {
    case 0:
      // C0
      CHECK_RECUR(L);
      LEFT;
      goto C;

    case 1:
      // C1
      WRITE(3);
      RIGHT;
      goto D;

    case 2:
      // C2
      WRITE(0);
      RIGHT;
      goto D;

    case 3:
      // C3
      WRITE(2);
      RIGHT;
      goto E;
  }

 D:
  switch (SCAN) {
    case 0:
      // D0
      WRITE(1);
      LEFT;
      goto F;

    case 1:
      // D1
      LEFT;
      goto C;

    case 2:
      // D2
      WRITE(1);
      RIGHT;
      goto B;
  }

 E:
  // E1
  assert(SCAN == 1);
  WRITE(3);
  RIGHT;
  goto D;

 F:
  switch (SCAN) {
    case 0:
      // F0
      WRITE(1);
      LEFT;
      goto G;

    case 2:
      // F2
      RIGHT;
      goto B;

    case 3:
      // F3
      WRITE(1);
      LEFT;
      goto F;
  }

 G:
  switch (SCAN) {
    case 0:
      // G0
      WRITE(2);
      RIGHT;
      goto E;

    case 2:
      // G2
      LEFT;
      goto C;
  }

 H:
  CHECK_STEPS;
}
