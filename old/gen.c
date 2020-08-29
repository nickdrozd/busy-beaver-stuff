#include <stdio.h>

char COLOR[] = {'1', '0'};
char SHIFT[] = {'L', 'R'};
char STATE[] = {'A', 'B', 'C', 'D'};

unsigned int COLORS = 2;
unsigned int SHIFTS = 2;
unsigned int STATES = 4;

unsigned int STEPS = 131072;  // 2^17

unsigned int c1, c2, s1, s2, q1, q2;

int main () {
  for (c1 = 0; c1 < COLORS; c1++) {
    for (c2 = 0; c2 < COLORS; c2++) {
      for (s1 = 0; s1 < SHIFTS; s1++) {
        for (s2 = 0; s2 < SHIFTS; s2++) {
          for (q1 = 0; q1 < STATES; q1++) {
            for (q2 = 0; q2 < STATES; q2++) {
              printf("%c%c%c %c%c%c\n",
                     COLOR[c1],
                     SHIFT[s1],
                     STATE[q1],
                     COLOR[c2],
                     SHIFT[s2],
                     STATE[q2]);
            }
          }
        }
      }
    }
  }
}
