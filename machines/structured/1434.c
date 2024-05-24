#define PROGRAM "1RB ... ... ...  0LC 2LC ... ...  0LC 3RD 0RD 2RE  1LF 1LC 1RB ...  ... 3RD ... ...  1LG ... 2RB 1LF  2RE ... 2LC ..."

#define TAPELEN 1000

#include "machine.h"

int main(void)
{
  // A0
  WRITE(1);
  RIGHT;

  // B0 / B1
  LEFT;

  assert(SCAN == 1);
  // C1
  WRITE(3);
  RIGHT;

 D:
  switch (SCAN) {
    case 1:
      // D1
      LEFT;
      goto C;

    case 2:
      // D2
      WRITE(1);
      RIGHT;

      // B1
      if (SCAN == 1) {
        WRITE(2);
      }
      // B0 / B1
      LEFT;
      goto C;

    case 0:
      // D0
      WRITE(1);
      LEFT;

      while (SCAN == 3) {
        // F3
        WRITE(1);
        LEFT;
      }

      switch (SCAN) {
        case 0:
          // F0
          WRITE(1);
          LEFT;

          if (BLANK) {
            // G0
            WRITE(2);
            RIGHT;

            // E1
            assert(SCAN == 1);
            WRITE(3);
            RIGHT;
            goto D;
          }

          // G2
          assert(SCAN == 2);
          LEFT;
          goto C;

        case 2:
          // F2
          RIGHT;

          // B1
          if (SCAN == 1) {
            WRITE(2);
          }
          // B0 / B1
          LEFT;
          goto C;
      }
  }

 C:
  while (BLANK) {
    // C0
    CHECK_RECUR(L);
    LEFT;
  }

  switch (SCAN) {
    case 1:
      // C1
      WRITE(3);
      break;

    case 2:
      // C2
      WRITE(0);
      break;

    case 3:
      // C3
      WRITE(2);
      RIGHT;

      // E1
      assert(SCAN == 1);
      WRITE(3);
      break;
  }

  RIGHT;
  goto D;

 H:
  CHECK_STEPS;
}
