module BB

public export
data RunType = Single | DoubleRec

public export
Programs : Type
Programs = (Nat, Nat, RunType, List (String, (Nat, Nat, Nat)))

public export
p2_2 : Programs
p2_2 = (2, 2, Single, [
  ("1RB 0LB  1LB 1LA", (6, 4, 2)),
  ("1RB 1LB  1LA 1R_", (6, 4, 4)),
  ("1RB 1LB  0LB 1LA", (6, 4, 2)),
  ("1RB 0RA  1LB 1LA", (8, 4, 0))])

public export
p3_2 : Programs
p3_2 = (3, 2, Single, [
  ("1RB 1R_  1LB 0RC  1LC 1LA", (21, 5, 5)),
  -- ("1RB 0LB  1LA 0RC  1LC 1LA", (57, 9, 8)), -- skip
  ("1RB 1LB  1LA 1LC  1RC 0LC", (34, 8, 0))])

public export
d3_2 : Programs
d3_2 = (3, 2, DoubleRec, [
  ("1RB 1RC  1LC 1RA  1RA 1LA", ( 9, 6, 6)),
  ("1RB 1RC  1LC 0LB  1RA 1LA", (22, 7, 5))])

public export
p2_3 : Programs
p2_3 = (2, 3, Single, [
  ("1RB 2RA 2LB  0LB 1LA 1RA", (23, 6, 4)),
  ("1RB 2LB 1R_  2LA 2RB 1LB", (38, 9, 9)),
  ("1RB 2LB 1LA  2LB 2RA 0RA", (59, 9, 8)),
  ("1LB 2RB 1RA  2RB 2LA 0LA", (59, 9, 8)),
  ("1RB 2LA 0RB  1LA 0LB 1RA", (77, 9, 0))])

public export
d2_3 : Programs
d2_3 = (2, 3, DoubleRec, [
  ("1RB 1LA 2RA  2LA 2LB 2RB", (16, 8, 7))])

public export
s4_2 : Programs
s4_2 = (4, 2, Single, [
  ("1RB 1LB  1LA 0LC  1R_ 1LD  1RD 0RA", (107, 14, 13)),
  ("1RB 0LC  1LD 0RC  1RA 0RB  0LD 1LA", (1459, 44, 25)),
  ("1RB 1RC  1LC 1RD  1RA 1LD  0RD 0LB", (2819, 70, 69)),
  ("1RB 0LC  1LD 0LA  1RC 1RD  1LA 0LD", (66345, 520, 0))])

public export
l4_2 : Programs
l4_2 = (4, 2, Single, [
  ("1RB 1LC  1RD 1RB  0RD 0RC  1LD 1LA", (32779477, 10240, 0))])

public export
s2_4 : Programs
s2_4 = (2, 4, Single, [
  ("1RB 2RA 3LA 1LB  0LB 2LA 3RA 1RB", (2476, 33, 31)),
  -- ("1RB 2RB 1LA 0LB  2LB 3RB 0RB 1LA", (32851, 194, 192)), -- skip
  ("1RB 2RA 1RA 2RB  2LB 3LA 0RB 2LA", (190524, 518, 0))])

public export
d2_4 : Programs
d2_4 = (2, 4, DoubleRec, [
  ("1RB 2LA 1RA 1LA  3LA 1LB 2RB 2LA", (6362, 84, 84)),
  ("1RB 2LA 1RA 1LA  3LA 1LB 2RB 2RA", (7106, 90, 90)),
  ("1RB 2LA 1RA 1LA  0LB 3LA 2RB 3RA", (9699, 79, 77)),
  ("1RB 2LB 2RA 3LA  1LA 3RA 3LB 0LB", (21485, 142, 142))])

public export
l2_4 : Programs
l2_4 = (2, 4, Single, [
  ("1RB 2RB 1LB 1LA  1LB 3RA 3LA 2RB", (2333909, 3341, 3340)),
  ("1RB 2LB 3RA 2LA  3LB 3RA 0RB 1RB", (2501552, 2748, 2747)),
  ("1RB 2LA 1RA 1RA  1LB 1LA 3RB 1R_", (3932964, 2050, 2050))])

public export
s5_2 : Programs
s5_2 = (5, 2, Single, [
  ("1RB 1LC  0LC 0RD  1RD 1LE  1RE 1LA  1LA 0LB", (31315, 142, 0))])

public export
l5_2 : Programs
l5_2 = (5, 2, Single, [
  ("1RB 1LC  1RD 0LE  0RD 0RC  1LD 1LA  1RB 1RE", (32810047, 10240, 0)),
  ("1RB 1LC  1RC 1RB  1RD 0LE  1LA 1LD  1R_ 0LA", (47176870, 12289, 4098))])

public export
p6_2 : Programs
p6_2 = (6, 2, Single, [
  ("1RB 1LE  1RD 1RB  0RD 0RE  1LD 1LA  0RF 1RF  0LC 1LC", (65538549, 10240, 0))])

public export
p3_3 : Programs
p3_3 = (3, 3, Single, [
  ("1RB 2RA 2RC  1LC 1R_ 1LA  1RA 2LB 1LC", (310341163, 36089, 36089)),
  ("1RB 1R_ 2RB  1LC 0LB 1RA  1RA 2LC 1RC", (4939345068, 107901, 107900)),
  ("1RB 2LA 1RA  1RC 2RB 0RC  1LA 1R_ 1LA", (987522842126, 1525690, 1525688)),
  ("1RB 1R_ 2LC  1LC 2RB 1LB  1LA 2RC 2LA", (4144465135614, 2950149, 2950149))])

public export
p2_5 : Programs
p2_5 = (2, 5, Single, [
  ("1RB 2RA 1LA 1LB 3LB  2LA 3RB 1R_ 4RA 1LA", (417310842648366, 36543045, 36543045))
])

public export
p10_4 : Programs
p10_4 = (10, 4, Single, [
  ("1RB 1LC 2LC 1RD  1LC 3LE 0LE 1LF  3RG 0LH 1RG 2R_  1RG 1RD 1RB 0LH  3R_ 2LC 1R_ 1RB  1RD 1LF 1LC 1LH  2LH 1LF 2LF 1LH  3RD 3LE 1RD 1LF  2R_ 3LH 0LH 1R_  3RB 1RD 1RB 3RD", (23587667, 6145, 4097))])

public export
p10_8 : Programs
p10_8 = (10, 8, Single, [
  ("1RB 1LC 2LC 3LC 4LC 2LC 1RD 1RE  2LF 1LG 2LG 3LG 4LG 3LF 4LF 1LF  5RB 2LC 3RB 4R_ 1RB 6R_ 7RB 1RB  4LC 7LH 0LH 5LH 6LH 1LC 4LG 1LG  7LH 1RE 1RB 2LC 1RD 1LG 0LF 5LF  5RE 5LF 3RE 1LG 1RE 1LF 7RE 3R_  7RE 1LG 1RE 1LC 7LH 1RE 1LG 1LF  5R_ 6LH 3R_ 1RD 1R_ 7RD 7R_ 4LG  2R_ 7LF 0LF 5LF 6LF 3R_ 4R_ 1R_  5RD 1RE 3RD 3RE 1RD 5RE 7RD 7RE", (15721562, 4097, 4097))])

public export
p6_9 : Programs
p6_9 = (6, 9, Single, [
  ("1LB 2RA 3LD 1LB 2RC 1R_ 1LD 4LD 1RC  5RC 3RA 1LB 2RE 3RC 6RC 2RA 2RC 5RA  7LD 1RC 3R_ 1R_ 1LB 2LD 1R_ 4R_ 6LD  8RC 3RE 1LD 2RA 1RC 1RC 2RE 6RC 5RE  3LD 2RE 3LB 1LD 6RC 6R_ 1LB 4LB 6RC  1R_ 1RC 6R_ 1R_ 1LB 8R_ 6RC 3LD 8RC", (2172583337460880, 47762041, 47762040))])
