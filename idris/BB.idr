module BB

public export
data RunType = Single | DoubleRec

public export
Programs : Type
Programs = (Nat, Nat, RunType, List (String, (Nat, Nat)))

public export
p2_2 : Programs
p2_2 = (2, 2, Single, [
  ("1RB 0LB  1LB 1LA", (6, 2)),
  ("1RB 1LB  1LA 1R_", (6, 4)),
  ("1RB 1LB  0LB 1LA", (6, 2)),
  ("1RB 0RA  1LB 1LA", (8, 0))])

public export
p3_2 : Programs
p3_2 = (3, 2, Single, [
  ("1RB 1R_  1LB 0RC  1LC 1LA", (21, 5)),
  -- ("1RB 0LB  1LA 0RC  1LC 1LA", (57, 8)), -- skip
  ("1RB 1LB  1LA 1LC  1RC 0LC", (34, 0))])

public export
d3_2 : Programs
d3_2 = (3, 2, DoubleRec, [
  ("1RB 1RC  1LC 1RA  1RA 1LA", ( 9, 6)),
  ("1RB 1RC  1LC 0LB  1RA 1LA", (21, 4))])

public export
p2_3 : Programs
p2_3 = (2, 3, Single, [
  ("1RB 2RA 2LB  0LB 1LA 1RA", (23, 4)),
  ("1RB 2LB 1R_  2LA 2RB 1LB", (38, 9)),
  ("1RB 2LB 1LA  2LB 2RA 0RA", (59, 8)),
  ("1LB 2RB 1RA  2RB 2LA 0LA", (59, 8)),
  ("1RB 2LA 0RB  1LA 0LB 1RA", (77, 0))])

public export
d2_3 : Programs
d2_3 = (2, 3, DoubleRec, [
  ("1RB 1LA 2RA  2LA 2LB 2RB", (17, 8))])

public export
s4_2 : Programs
s4_2 = (4, 2, Single, [
  ("1RB 1LB  1LA 0LC  1R_ 1LD  1RD 0RA", (107, 13)),
  ("1RB 0LC  1LD 0RC  1RA 0RB  0LD 1LA", (1459, 25)),
  ("1RB 1RC  1LC 1RD  1RA 1LD  0RD 0LB", (2819, 69)),
  ("1RB 0LC  1LD 0LA  1RC 1RD  1LA 0LD", (66345, 0))])

public export
l4_2 : Programs
l4_2 = (4, 2, Single, [
  ("1RB 1LC  1RD 1RB  0RD 0RC  1LD 1LA", (32779477, 0))])

public export
s2_4 : Programs
s2_4 = (2, 4, Single, [
  ("1RB 2RA 3LA 1LB  0LB 2LA 3RA 1RB", (2476, 31)),
  -- ("1RB 2RB 1LA 0LB  2LB 3RB 0RB 1LA", (32851, 192)), -- skip
  ("1RB 2RA 1RA 2RB  2LB 3LA 0RB 2LA", (190524, 0))])

public export
d2_4 : Programs
d2_4 = (2, 4, DoubleRec, [
  ("1RB 2LA 1RA 1LA  3LA 1LB 2RB 2LA", (6362, 84)),
  ("1RB 2LA 1RA 1LA  3LA 1LB 2RB 2RA", (7106, 90)),
  ("1RB 2LA 1RA 1LA  0LB 3LA 2RB 3RA", (9699, 77)),
  ("1RB 2LB 2RA 3LA  1LA 3RA 3LB 0LB", (21485, 142))])

public export
l2_4 : Programs
l2_4 = (2, 4, Single, [
  ("1RB 2RB 1LB 1LA  1LB 3RA 3LA 2RB", (2333909, 3340)),
  ("1RB 2LB 3RA 2LA  3LB 3RA 0RB 1RB", (2501552, 2747)),
  ("1RB 2LA 1RA 1RA  1LB 1LA 3RB 1R_", (3932964, 2050))])

public export
ll2_4 : Programs
ll2_4 = (2, 4, Single, [
  ("1RB 2RB 3LA 2RA  2LB 1LA 0RB 3RA", (   1012664081, 0)),
  ("1RB 2RA 1RA 2RB  2LB 3LA 0RB 0RA", (1367361263049, 0))])

public export
lll2_4 : Programs
lll2_4 = (2, 4, Single, [
  ("1RB 2RA 1LA 2LB  2LB 3RB 0RB 1RA", (67093892759901295, 530843045))])

public export
s5_2 : Programs
s5_2 = (5, 2, Single, [
  ("1RB 1LC  0LC 0RD  1RD 1LE  1RE 1LA  1LA 0LB", (31315, 0))])

public export
l5_2 : Programs
l5_2 = (5, 2, Single, [
  ("1RB 1LC  1RD 0LE  0RD 0RC  1LD 1LA  1RB 1RE", (32810047, 0)),
  ("1RB 1LC  1RC 1RB  1RD 0LE  1LA 1LD  1R_ 0LA", (47176870, 4098))])

public export
ll5_2 : Programs
ll5_2 = (5, 2, Single, [
  ("1RB ...  0LC 0LB  0LD 1LC  1RD 0RE  1LB 1LA", (455790469746, 0))])

public export
p6_2 : Programs
p6_2 = (6, 2, Single, [
  ("1RB 1LE  1RD 1RB  0RD 0RE  1LD 1LA  0RF 1RF  0LC 1LC", (65538549, 0))])

public export
p3_3 : Programs
p3_3 = (3, 3, Single, [
  ("1RB 2RA 2RC  1LC 1R_ 1LA  1RA 2LB 1LC", (310341163, 36089)),
  ("1RB 1R_ 2RB  1LC 0LB 1RA  1RA 2LC 1RC", (4939345068, 107900)),
  ("1RB 2LA 1RA  1RC 2RB 0RC  1LA 1R_ 1LA", (987522842126, 1525688)),
  ("1RB 1R_ 2LC  1LC 2RB 1LB  1LA 2RC 2LA", (4144465135614, 2950149))])

public export
p2_5 : Programs
p2_5 = (2, 5, Single, [
  ("1RB 2RA 1LA 1LB 3LB  2LA 3RB 1R_ 4RA 1LA", (417310842648366, 36543045))
])

public export
p8_4 : Programs
p8_4 = (8, 4, Single, [
  ("1RB ... ... ...  1LC ... 1LD ...  2RE 0LF ... ...  1RG 1LD 1LF ...  3LF 1LD ... 3LD  2RG 2LH 1LD ...  1RE 1RG ... 1RB  1R_ 3LC 1RB ...", (23587667, 4097))])

public export
p5_5 : Programs
p5_5 = (5, 5, Single, [
  ("1RB ... ... ... ...  2LC ... ... ... ...  3RD 3LC ... 1LC 1R_  ... 1RD 1RB 1LE ...  4RD 1LE ... 1RD 1LC", (15721562, 4097))])

public export
p7_7 : Programs
p7_7 = (7, 7, Single, [
  ("1RB ... ... ... ... ... ...  0LC 2LD ... ... ... 3LD ...  4RE 1RF ... ... ... ... ...  2RE 0LD 0LC ... 1RE ... ...  1RE 0LD 1RB 1LG 1RF 1LG 5LG  6LG 4LD ... ... ... 0LD 5LG  2RF 1LG 1LC ... 1RB ... ...", (10929166, 3414))])

public export
p6_9 : Programs
p6_9 = (6, 9, Single, [
  ("1LB 2RA 3LD 1LB 2RC 1R_ 1LD 4LD 1RC  5RC 3RA 1LB 2RE 3RC 6RC 2RA 2RC 5RA  7LD 1RC 3R_ 1R_ 1LB 2LD 1R_ 4R_ 6LD  8RC 3RE 1LD 2RA 1RC 1RC 2RE 6RC 5RE  3LD 2RE 3LB 1LD 6RC 6R_ 1LB 4LB 6RC  1R_ 1RC 6R_ 1R_ 1LB 8R_ 6RC 3LD 8RC", (2172583337460880, 47762040))])

-- derived from 5/2
-- 1RB 0LC  1RC 0LA  1LD 0RB  0RE 0RD  1LE 0LA
-- 2190942280098521917
public export
ll9_8 : Programs
ll9_8 = (9, 8, Single, [
  ("1RB ... ... ... ... ... ... ...  2LC 3LC 0RD 0RB 2RE ... ... 4RF  3LC 4LG 5LH 0RB 1LI 4RF 2RE ...  1LC 0LG ... 4LG ... ... ... ...  0RD 0LG 6LG 4LG 3RF 0RB 7RF ...  1LI 0RB 1RB 2RE 3LI 7LI ... ...  5RE 7RE 3RE 7RF 5RF 1RB 4RE ...  4RF 3RF ... 2RE 6RE 0RE ... 1RE  0RD ... 2RD 2RB 6RB 0RB 6RD 1RB", (730314092792526196, 0))])
