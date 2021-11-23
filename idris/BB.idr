module BB

public export
Programs : Type
Programs = (Nat, Nat, List String)

public export
p2_2 : Programs
p2_2 = (2, 2, [
  "1RB 1LB  1LA 1RH",
  "1RB 0RA  1LB 1LA"])

public export
p3_2 : Programs
p3_2 = (3, 2, [
  "1RB 1RH  1LB 0RC  1LC 1LA",
  "1RB 1LB  1LA 1LC  1RC 0LC"])

public export
p2_3 : Programs
p2_3 = (2, 3, [
  "1RB 2LB 1RH  2LA 2RB 1LB",
  "1RB 2LA 0RB  1LA 0LB 1RA"])

public export
p4_2 : Programs
p4_2 = (4, 2, [
  "1RB 1LB  1LA 0LC  1RH 1LD  1RD 0RA",
  "1RB 0LC  1LD 0LA  1RC 1RD  1LA 0LD",
  "1RB 1LC  1RD 1RB  0RD 0RC  1LD 1LA"])

public export
p2_4 : Programs
p2_4 = (2, 4, [
  "1RB 2RA 1RA 2RB  2LB 3LA 0RB 2LA",
  "1RB 2LA 1RA 1RA  1LB 1LA 3RB 1RH"])

public export
p5_2 : Programs
p5_2 = (5, 2, [
  "1RB 1LC  0LC 0RD  1RD 1LE  1RE 1LA  1LA 0LB",
  "1RB 1LC  1RD 0LE  0RD 0RC  1LD 1LA  1RB 1RE",
  "1RB 1LC  1RC 1RB  1RD 0LE  1LA 1LD  1RH 0LA"])

public export
p6_2 : Programs
p6_2 = (6, 2, [
  "1RB 1LE  1RD 1RB  0RD 0RE  1LD 1LA  0RF 1RF  0LC 1LC"])

public export
p3_3 : Programs
p3_3 = (3, 3, [
  "1RB 2RA 2RC  1LC 1RH 1LA  1RA 2LB 1LC",
  "1RB 1RH 2RB  1LC 0LB 1RA  1RA 2LC 1RC",
  "1RB 2LA 1RA  1RC 2RB 0RC  1LA 1RH 1LA",
  "1RB 1RH 2LC  1LC 2RB 1LB  1LA 2RC 2LA"])
