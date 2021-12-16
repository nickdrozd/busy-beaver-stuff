module BB

public export
Programs : Type
Programs = (Nat, Nat, List (String, (Nat, Nat, Nat)))

public export
p2_2 : Programs
p2_2 = (2, 2, [
  ("1RB 0LB  1LB 1LA", (6, 4, 2)),
  ("1RB 1LB  1LA 1RH", (6, 4, 4)),
  ("1RB 1LB  0LB 1LA", (6, 4, 2)),
  ("1RB 0RA  1LB 1LA", (8, 4, 0))])

public export
p3_2 : Programs
p3_2 = (3, 2, [
  ("1RB 1RH  1LB 0RC  1LC 1LA", (21, 5, 5)),
  -- ("1RB 0LB  1LA 0RC  1LC 1LA", (57, 9, 8)), -- skip
  ("1RB 1LB  1LA 1LC  1RC 0LC", (34, 8, 0))])

public export
p2_3 : Programs
p2_3 = (2, 3, [
  ("1RB 2RA 2LB  0LB 1LA 1RA", (23, 6, 4)),
  ("1RB 2LB 1RH  2LA 2RB 1LB", (38, 9, 9)),
  ("1RB 2LB 1LA  2LB 2RA 0RA", (59, 9, 8)),
  ("1LB 2RB 1RA  2RB 2LA 0LA", (59, 9, 8)),
  ("1RB 2LA 0RB  1LA 0LB 1RA", (77, 9, 0))])

public export
s4_2 : Programs
s4_2 = (4, 2, [
  ("1RB 1LB  1LA 0LC  1RH 1LD  1RD 0RA", (107, 14, 13)),
  ("1RB 0LC  1LD 0RC  1RA 0RB  0LD 1LA", (1459, 44, 25)),
  ("1RB 1RC  1LC 1RD  1RA 1LD  0RD 0LB", (2819, 70, 69)),
  ("1RB 0LC  1LD 0LA  1RC 1RD  1LA 0LD", (66345, 520, 0))])

public export
l4_2 : Programs
l4_2 = (4, 2, [
  ("1RB 1LC  1RD 1RB  0RD 0RC  1LD 1LA", (32779477, 10240, 0))])

public export
s2_4 : Programs
s2_4 = (2, 4, [
  ("1RB 2RA 3LA 1LB  0LB 2LA 3RA 1RB", (2476, 33, 31)),
  -- ("1RB 2RB 1LA 0LB  2LB 3RB 0RB 1LA", (32851, 194, 192)), -- skip
  ("1RB 2RA 1RA 2RB  2LB 3LA 0RB 2LA", (190524, 518, 0))])

public export
l2_4 : Programs
l2_4 = (2, 4, [
  ("1RB 2RB 1LB 1LA  1LB 3RA 3LA 2RB", (2333909, 3341, 3340)),
  ("1RB 2LB 3RA 2LA  3LB 3RA 0RB 1RB", (2501552, 2748, 2747)),
  ("1RB 2LA 1RA 1RA  1LB 1LA 3RB 1RH", (3932964, 2050, 2050))])

public export
s5_2 : Programs
s5_2 = (5, 2, [
  ("1RB 1LC  0LC 0RD  1RD 1LE  1RE 1LA  1LA 0LB", (31315, 142, 0))])

public export
l5_2 : Programs
l5_2 = (5, 2, [
  ("1RB 1LC  0LC 0RD  1RD 1LE  1RE 1LA  1LA 0LB", (31315, 142, 0)),
  ("1RB 1LC  1RD 0LE  0RD 0RC  1LD 1LA  1RB 1RE", (32810047, 10240, 0)),
  ("1RB 1LC  1RC 1RB  1RD 0LE  1LA 1LD  1RH 0LA", (47176870, 12289, 4098))])

public export
p6_2 : Programs
p6_2 = (6, 2, [
  ("1RB 1LE  1RD 1RB  0RD 0RE  1LD 1LA  0RF 1RF  0LC 1LC", (65538549, 10240, 0))])

public export
p3_3 : Programs
p3_3 = (3, 3, [
  ("1RB 2RA 2RC  1LC 1RH 1LA  1RA 2LB 1LC", (310341163, 36089, 36089)),
  ("1RB 1RH 2RB  1LC 0LB 1RA  1RA 2LC 1RC", (4939345068, 107901, 107900)),
  ("1RB 2LA 1RA  1RC 2RB 0RC  1LA 1RH 1LA", (987522842126, 1525690, 1525688)),
  ("1RB 1RH 2LC  1LC 2RB 1LB  1LA 2RC 2LA", (4144465135614, 2950149, 2950149))])

public export
p2_5 : Programs
p2_5 = (2, 5, [
  ("1RB 2RA 1LA 1LB 3LB  2LA 3RB 1RH 4RA 1LA", (417310842648366, 36543045, 36543045))
])
