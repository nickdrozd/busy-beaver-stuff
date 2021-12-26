# pylint: disable = attribute-defined-outside-init

from unittest import TestCase, skip

from tm import run_bb
from generate.program import Program

HALTING_FAST = {
    # 2/2 BB
    "1RB 1LB  1LA 1R_": (4, 6),
    "1RB 0LB  1LA 1R_": (3, 6),
    "1RB 1R_  1LB 1LA": (3, 6),
    "1RB 1R_  0LB 1LA": (2, 6),

    # 3/2 BB
    "1RB 1R_  1LB 0RC  1LC 1LA": (5, 21),  # shift
    "1RB 1R_  0LC 0RC  1LC 1LA": (5, 20),
    "1RB 1LA  0RC 1R_  1LC 0LA": (5, 20),
    "1RB 1RA  0RC 1R_  1LC 0LA": (5, 19),
    "1RB 0RA  0RC 1R_  1LC 0LA": (4, 19),
    "1RB 1LC  1RC 1R_  1LA 0LB": (6, 11),  # sigma

    # 2/3 BB
    "1RB 2LB 1R_  2LA 2RB 1LB": (9, 38),
    "1RB 0LB 1R_  2LA 1RB 1RA": (8, 29),
    "0LB 2RB 1R_  1RA 1LB 1LA": (6, 27),
    "1RB 1LA 1LB  0LA 2RA 1R_": (6, 26),
    "1RB 2LA 1R_  1LB 1LA 0RA": (6, 26),

    # 4/2 BB
    "1RB 1LB  1LA 0LC  1R_ 1LD  1RD 0RA": (13, 107),  # shift
    "1RB 1LC  1LD 0RB  1R_ 0LD  1RA 1LA": ( 9,  97),
    "1RB 0RC  1LA 1RA  1R_ 1RD  1LD 0LB": (13,  96),  # sigma
    "1RB 1LB  0LC 0RD  1R_ 1LA  1RA 0LA": ( 6,  96),
    "1RB 1LC  0LD 0RD  1R_ 0LA  1LD 1LA": (11,  84),

    # 2/4 Runners-up
    "1RB 2LA 1RA 1LA  3LA 1R_ 2RB 2RA": (90, 7195),
    "1RB 2LA 1RA 1LA  3LA 1R_ 2RB 2LA": (84, 6445),
    "1RB 2LA 1RA 1LA  3LA 1R_ 2RB 3RA": (84, 6445),
    "1RB 2RB 3LA 2RA  1LA 3RB 1R_ 1LB": (60, 2351),

    # Milton Green (1964)
    "1RB 1LA  0L_ 1RB": (1, 2),
    "1RB 1L_  0RC 1RC  0RD 0RC  1RE 1LA  0RF 0RE  1LF 1LD": (35, 436),

    # Lynn (1971)
    "1RB 1RA  1LC 0LD  0RA 1LB  1R_ 0LE  1RC 1RB": (15, 435),
    "1RB 1RC  1LC 1LD  0RA 1LB  1RE 0LB  1R_ 1RD": (22, 292),
    "1RB 0RC  1LC 0LB  1RD 1LB  1RE 0RA  0RB 1R_": (22, 217),
    # Lynn reports 522 steps
    "1RB 0LB  1LC 1R_  0LD 0LC  1LE 0RA  0LF 0LE  1RF 1RD": (42, 521),

    # Uwe (1981)

    # Castor diligentissimus et primus et perpetuus (Castor schultis)
    "1RB 0LC  1RC 1RD  1LA 0RB  0RE 1R_  1LC 1RA": (501, 134467),

    # Castor ministerialis: the Civil Servant Beaver, who cares most
    # for his progress, but does not produce anything.
    "1RB 1RA  1RC 0RD  1LE 0RA  0R_ 0RB  1LB 1LE": (0, 52),

    # Castor scientificus: the Scientific Beaver, who does not produce
    # anything either, but with more effort and less effect on his
    # position.
    "0RB 0LA  0RC 0R_  1RD 1LE  1LA 0LD  1RC 1RE": (0, 187),

    # Castor exflippus: the Beaver Freak, who tries to survive as long
    # as possible without producing anything, moving on the tape, and
    # changing his state.
    "0RB 0LA  1RC 0R_  0LC 1RD  0LD 1RE  1LA 0LE": (0, 67),

    # 3/3

    # R. Blodgett
    "1RB 1LB 2LB  1RC 1LA 0RA  1LA 2RA 1R_": (9, 57),

    # David Jefferson
    "1RB 1RA 1R_  1LC 1LC 2LA  2RA 1LB 1LA": (12, 44),
}

HALTING_SLOW = {
    # 3/3 Surprise-in-a-box
    "1RB 2LB 1LC  1LA 2RB 1RB  1R_ 2LA 0LC": (31, 2315619),

    # 2/4 BB
    "1RB 2LA 1RA 1RA  1LB 1LA 3RB 1R_": (2050, 3932964),

    # 3/3 copy of 2/4 BB
    "1RB 1LC 1R_  1LA 1LC 2RB  1RB 2LC 1RC": (2050, 3932964),

    # 5/2 BB
    "1RB 1LC  1RC 1RB  1RD 0LE  1LA 1LD  1R_ 0LA": (4098, 47176870),

    # # 3/3 Brady 2004 (not BB)
    # "1RB 1R_ 2LC  1LC 2RB 1LB  1LA 0RB 2LA": (13949, 92649163),
}

QUASIHALTING = {
    # 2/2 (not better than BB)
    "1RB 1LB  1LB 1LA": (3, 6, 1),
    "1RB 0LB  1LB 1LA": (2, 6, 1),

    # 3/2
    "1RB 0LB  1LA 0RC  1LC 1LA": (6, 55, 1),  # BBB shift
    "1RB 0LB  1RC 0RC  1LC 1LA": (6, 54, 1),
    "1RB 0LC  1LB 0RC  1LC 1LA": (5, 52, 1),  # BB extension
    "1RB 0LC  0LC 0RC  1LC 1LA": (5, 51, 1),
    "1RB 0LC  1LA 0RC  1RC 1RB": (5, 49, 1),
    "1RB 0LC  0RC 0RC  1LC 1LA": (5, 48, 1),
    "1RB 1LB  1LA 1LC  1RC 0LC": (0, 34, 1),
    "1RB 1LC  0RC ...  1LC 0LA": (5, 27, 1),
    "1RB 1LC  1LB 1LA  1RC 0LC": (0, 27, 1),
    "1RB 1LB  1LA 1RC  1LC 0RC": (0, 26, 1),
    "1RB ...  1LB 0LC  1RC 1RB": (3, 5, 13),
    "1RB ...  1LB 1RC  0LC 0RB": (2, 2, 14),
    "1RB ...  1LB 1LC  1RC 0RB": (2, 2, 13),
    "1RB ...  1LC 0RB  1LB 1RC": (2, 2, 10),

    # 2/3
    "1RB 2LB 1LA  2LB 2RA 0RA": ( 8, 59, 1),  # BBB shift
    "1RB 0LB 1RA  1LB 2LA 2RA": ( 3, 45, 1),
    "1RB 2LB 1RA  2LB 2LA 0RA": (10, 43, 1),  # BBB sigma
    "1RB 2RA 2LB  2LB 2LA 0LA": ( 5, 40, 1),
    "1RB 1LB 1RA  2LB 2LA 0RA": ( 6, 23, 1),
    "1RB 2LB ...  1LB 2LA 1RB": ( 5, 17, 1),
    "1RB 2LA 1RA  2LB 1LA 2RB": ( 5, 16, 3),
    "1RB ... ...  2LB 1RB 1LB": ( 1,  1, 5),

    # 4/2
    "1RB 0LC  1LD 0LA  1RC 1RD  1LA 0LD": (0, 66349, 1),
    # TNF: "1RB 0LD  1LC 0LA  1LA 0LC  1RD 1RC"
    "1RB 1RA  0RC 0RB  0RD 1RA  1LD 1LB": ( 0, 2568, 1),
    "1RB 1RA  0RC 1LA  1LC 1LD  0RB 0RD": ( 0, 2512, 1),
    "1RB 1RC  1RD 0LC  1LD 0LD  1LB 0RA": (56, 2332, 3),
    "1RB 0LC  1RC 1LD  1RD 0RB  0LB 1LA": (35, 1460, 3),  # QH 1459
    "1RB 1LC  1LC 0RD  1LA 0LB  1LD 0RA": (39, 1164, 1),
    "1RB 1LB  1RC 0LD  0RD 0RA  1LD 0LA": (20, 1153, 1),
    "1RB 0LB  0RC 0LC  0RD 1LC  1LD 0LA": (19,  673, 1),
    "1RB 0LC  1LD 0RA  1RC 1RB  1LA 0LB": (31,  651, 1),
    "1RB 0LC  0RD 1LC  0LA 1LB  1LD 0RB": (22,  536, 1),
    "1RB 0LB  1LB 1LC  1RD 0LB  1RA 0RD": (12,  444, 1),
    "1RB 0RC  0RD 1RA  0LD 0LA  1LC 1LA": ( 8,  334, 2),
    "1RB 0RB  1LC 1RA  0LD 1LB  1RD 0LB": ( 8,  119, 6),
    "1RB 1LC  1LD 0RA  1RC 0LD  0LC 1LA": (10,  108, 8),
    "1RB 0LC  0RD 1RC  1LA 1RD  1LD 0RB": (10,  105, 8),
    "1RB 1LA  1RC 1LD  1RD 0RC  1LB 0LA": ( 7,  101, 8),
    # D-initial version of BLB(4) champ
    "1LA 1LB  1RC 1LD  1RA 1RC  0RA 0RD": ( 0,    0, 1),

    # 5/2
    "1RB 1LC  1LC 1RA  1LB 0LD  1LA 0RE  1RD 1RE": (504, 221032, 2),
    "1RB 1LC  0LC 0RD  1RD 1LE  1RE 1LA  1LA 0LB": (  2,  31317, 3),
    "1RB 1LC  1RD 1RA  1LB 0LA  1RE 0RC  1RC 0LE": (  2,   3247, 3),

    # 2/4
    "1RB 2RA 1RA 2RB  2LB 3LA 0RB 2LA": (   0,  190524, 1),
    "1RB 2RB 1LA 0LB  2LB 3RB 0RB 1LA": ( 190,   32849, 1),
    "1RB 2RB 3LA 2RA  1LB 1LA 1LB 3RB": (  62,   22464, 1),  # QH 22402
    "1RB 2RA 3LA 0LB  1LB 1LA 0RB 1RB": (  99,   16634, 1),
    "1RB 2LA 1RA 1LA  2LB 3LA 2RB 2RA": ( 106,   10456, 3),  # QH 10353
    "1RB 2RB 1LA 1LA  2LB 2RA 3LB 1LA": (  62,    4067, 1),  # QH 4005
    "1RB 2RB 3LA 2RA  1LB 1LA 1LB 3RA": (  42,    3247, 1),
    "1RB 2RB 3LA 2RA  1LB 1LA 2LB 3RA": (  42,    3057, 1),
    "1RB 2RA 3LB 2LA  1LB 3LA 3RA 1RB": (  44,    3054, 1),
    "1RB 2LB 3RA 0LA  1LB 2RB 2LA 1LA": (  31,    2872, 1),
    "1RB 0LA 1RA 0LB  2LB 3LA 2RB 0RA": (  57,    2859, 3),
    "1RB 0RB 0LA 2LB  1LB 2LA 3RB 1RA": (  32,    1769, 1),
    "1RB 0LA 0RB 2LB  3LB 3RA 0RA 1LA": (  36,    1525, 1),
    "1RB 0LA 0RB 2LB  3LB 3RA 1RB 1LA": (  35,    1458, 1),

    # "1RB 3LA 1LA 1RA  2LB 2RA 0RB 3RB" -- QH 77, xmas
    # "1RB 2LA 2RB 1LA  3LB 3RA 2RB 0RB" -- QH 14, xmas
}

QUASIHALTING_FIXED = {
    # 2/2
    "1RB 1LB  0LB 1LA": (2, 6, 1),
    "1RB 0LB  0LB 1LA": (1, 6, 1),

    # 3/2
    "1RB 1RC  1LC 0LB  1RA 1LA": (5, 22, 2),  # center, >BB
    "1RB 1RC  1LC 1RA  1RA 1LA": (6,  9, 2),  # center, >BB sigma

    # 2/3
    "1RB 2RA 2LB  0LB 1LA 1RA": ( 4, 23, 1),
    "1RB 1LA 2RA  2LA 2LB 2RB": ( 8, 17, 2),

    # 4/2
    "1RB 1RC  1LC 1RD  1RA 1LD  0RD 0LB": (69, 2819, 1),  # BBB sigma
    "1RB 0LC  1LD 0RC  1RA 0RB  0LD 1LA": (25, 1459, 1),
    "1RB 1RC  1LD 0RA  0RC 1RD  1RA 0LB": (32,  581, 1),

    # 2/4
    "1RB 2LB 2RA 3LA  1LA 3RA 3LB 0LB": (142, 21485, 2),
    "1RB 2LA 1RA 1LA  0LB 3LA 2RB 3RA": ( 77,  9698, 2),  # QH 9623
    "1RB 2LA 1RA 1LA  3LA 1LB 2RB 2RA": ( 90,  7193, 2),  # QH 7106
    "1RB 2LA 1RA 1LA  3LA 1LB 2RB 2LA": ( 84,  6443, 2),  # QH 6362
    "1RB 2RA 3LA 1LB  0LB 2LA 3RA 1RB": ( 31,  2476, 1),
    "1RB 2RA 2LA 3LB  0LB 1LA 3RB 0RA": ( 30,  1854, 1),
}

QUASIHALTING_SLOW = {
    # 4/2
    "1RB 1LC  1RD 1RB  0RD 0RC  1LD 1LA": (0, 32779478, 1),

    # 2/4
    "1RB 2LB 3RA 2LA  3LB 3RA 0RB 1RB": (2747, 2501552, 1),
    "1RB 2RB 1LB 1LA  1LB 3RA 3LA 2RB": (3340, 2333909, 1),
}

RECURRENCE_FAST = {
    # Lin-Rado examples
    "1RB ...  0RC 1LB  1LA 0RB": (2,  9, 10),  # total recurrence
    "1RB ...  1LB 0LC  1LA 1RA": (4, 12,  7),  # left barrier
    "1RB ...  1LC 1RA  1LA 0LC": (4, 12,  8),  # right barrier

    # 2/2
    "1RB 0LB  1LA 0RB": (3, 9, 3),
    "1RB 1LA  0LA 1RA": (3, 7, 5),
    "1RB 1LB  1LA 0RB": (2, 7, 3),
    "1RB 1RB  1LA 0LB": (2, 3, 4),
    "1RB 0RB  1LB 1RA": (0, 0, 9),
    "1RB 0RA  1LB 1LA": (0, 0, 8),
    "1RB 0RA  0LB 1LA": (0, 0, 7),
    "1RB 1LA  0LA 0LB": (0, 0, 7),
    "1RB 1LA  1LB 0RA": (0, 0, 6),
    "1RB 0LA  1LB 1RA": (0, 0, 5),

    # 3/2
    "1RB 1LB  0RC 0LA  1LC 0LA": ( 9, 101, 24),
    "1RB 1LA  1LC 1RC  1LA 0RB": (10,  69, 16),
    "1RB 1LB  1RC 0LA  1LA 1RC": (10,  65, 16),
    "1RB 0LC  1LC 1RB  1RA 1LA": ( 9,  50, 16),
    "1RB 1LC  1LA 1RB  1RB 0LA": ( 9,  50, 12),
    "1RB 0LC  1LC 1RB  1RB 1LA": ( 9,  50, 12),
    "1RB 0LB  1LC 0RC  1RA 1LA": ( 6,  38, 21),
    "1RB 1LB  1LA 1RC  0RB 0LC": ( 0,  22,  4),
    "1RB 1LA  0RC 0RA  1LC 0LA": ( 4,  17, 36),
    "1RB 0RB  1LC 1RC  0LA 1LA": ( 3,  16, 15),
    "1RB ...  1LC 0RC  1RA 0LC": ( 4,  16,  5),
    "1RB 1LB  0RC 0RB  1LC 0LA": ( 3,   4, 38),
    "1RB 0RB  1LC 0RC  0LA 1RA": ( 2,   2, 30),
    "1RB 0LA  0RC 1LA  1LC 0RB": ( 0,   0, 92),
    "1RB 0LA  1LB 0RC  1LC 1LA": ( 0,   0, 56),
    "1RB 0LA  0RC 0RC  1LC 1LA": ( 0,   0, 48),
    "1RB 1LB  0RC 1LA  1LA 0RA": ( 0,   0, 21),

    # 2/3
    "1RB 0LA ...  1LB 2LA 0RB": (15, 165, 54),
    "1RB 1LB 2LA  1LA 2RB 0RA": (12, 101, 26),
    "1RB 2RB 1LB  1LA 2RB 0LA": (13,  97, 14),
    "1RB 2LA 0RB  1LA 1RB 1RA": (13,  94, 20),
    "1RB 2LA 0RB  1LA 2LB 1RA": (11,  89, 26),
    "1RB 1LA 1LB  1LA 2RB 0LA": (12,  80, 20),
    "1RB 2LA 0RB  1LA 2LA 1RA": (12,  78, 14),
    "1RB 2LA 0RB  1LB 2LA 1RA": (10,  76, 14),
    "1RB 2LA 0RB  1LA 0LB 1RA": ( 2,  75,  4),
    "1RB 2LB 2LA  2LA 0LB 0RA": ( 8,  63, 32),
    "1RB 0RA 2LB  2LA 2RA 0LB": ( 6,  59, 32),
    "1RB 1LB 1LB  1LA 2RB 0LA": ( 9,  58,  8),
    "1RB 2LA 2LB  1LA 2RA 0LB": ( 8,  57, 60),
    "1RB 1LA 2LB  2LA 2RA 0LB": ( 6,  57, 30),
    "1RB 2LA 0RB  1LB 1RA 1RA": ( 6,  55, 10),
    "1RB 0RB 0LB  2LA 2RA 1LB": ( 7,  54, 40),
    "1RB 2LA 0RB  2LA ... 1RA": ( 8,  35,  8),
    "1RB 2LA 1RB  1LB 1LA 2RA": ( 7,  24, 46),
    "1RB 1LA 2LB  1LA 2RA 0LB": ( 7,  20, 48),
    "1RB 2RB 2LA  1LB 1RA 0LA": ( 4,  14, 54),
    "1RB 0RB 1LA  2LA 2RA 0LB": ( 3,  10, 48),
    "1RB 2LA 1RB  1LB 1LA 0RA": ( 4,   7, 46),
    "1RB 0RA 1LB  2LA 2RB 0LA": ( 3,   6, 48),
    "1RB 0RA 2LB  2LA 0LA 1RA": ( 2,   5, 28),
    "1RB 1RA 0RB  2LB 1LA 1LB": ( 3,   4, 23),
    "1RB 2LA 0LB  1LA 2RA 2RB": ( 2,   3, 35),
    "1RB 2LA 0RB  0LB 1LA 0RA": ( 1,   2, 57),
    "1RB 0RB 0LB  1LB 2RA 1LA": ( 2,   2, 30),
    "1RB 2LB 2LA  1LA 2RB 0RA": ( 1,   1, 35),
    "1RB 2LB 0RA  1LA 2RB 2RA": ( 0,   0, 60),
    "1RB 2LB 0RA  1LA 1RB 2RA": ( 0,   0, 48),
    "1RB 2LA 1LB  0LA 0RB 1RA": ( 0,   0, 47),

    # 4/2
    "1RB 1RC  1LC 0LD  1RA 0LB  0RA 0RC": (124, 14008, 24),
    "1RB 1LC  0RC 0RD  1LA 0LA  0LC 1RB": (73, 7002,  225),
    "1RB 0RA  1RC 0LB  1LD 0RD  1RA 1LB": (85, 6836,  382),
    "1RB 0LC  0RC 1RC  1LA 0RD  1LC 0LA": (106, 6825, 342),
    "1RB 0LC  1RD 1LD  0LA 1LB  1LC 0RD": (52, 6455,   23),
    "1RB 0LC  0RD 1RD  0LA 1LC  1LA 0RA": (69, 5252,    9),
    "1RB 0LA  0RC 0RD  1LC 1LA  0RB 1RD": (70, 3957,  265),
    "1RB 0LC  0RD 1RD  1LA 1LC  1RC 0RB": (49, 3316,  208),
    "1RB 0RA  1RC 0LD  0LB 1RA  0LA 1LD": (32, 3115,  860),
    "1RB 0LB  1LA 0LC  1LB 0RD  1RC 0RB": (40, 2374,  359),
    "1RB 1RC  1LC 0RB  1LD 0RA  1RA 0LB": (51, 1727,  622),
    "1RB 0LC  1RD 0RD  1LA 0RC  1LB 1RC": (39, 1527,  522),
    "1RB 0LC  1RC 1RD  1LD 0RC  1LA 0RB": (45, 1301,  622),
    "1RB 1LC  1RD 0RB  0LC 1LA  1RC 0RA": (33, 1111,  131),
    "1RB 1RC  1LB 1LC  1RD 0LB  1RA 0RD": (30, 1033,  174),
    "1RB 0LC  1RD 0RB  1LC 1LA  1RC 1RA": (30, 1004,  174),
    "1RB 1LA  1RC 0RD  0LA 0RC  1RC 1LC": (29,  979,  144),
    "1RB 1RC  1LC 0LD  0RA 1LB  1RD 0LA": (24,  928,  128),
    "1RB 0RA  0LB 0LC  1RD 1LC  1RA 1LB": (19,  868,  404),
    "1RB 0RC  1LB 1RC  1RA 0LD  1LA 1LC": (23,  845,  842),
    "1RB 1RC  1LC 0RB  1RA 0LD  0LC 1LD": (22,  600, 1374),
    "1RB 1LA  1LC 0RA  1LD 0LC  1RB 0LA": (25,  497,  816),
    "1RB 0RC  0LD 1RA  0LA 0RD  1LC 1LA": (12,  383,  200),
    "1RB 0LA  1LC 1LD  1RD 1LB  1RA 0RD": (12,   79,  481),
    "1RB 0LC  0RD 0RC  1LD 0RB  1LA 0LC": ( 8,   74,  945),
    "1RB 0LC  1RD 0RA  0LB 0LA  1LC 0RA": ( 9,   67,  945),
    "1RB 1LA  1RC 0RC  1LD 0RD  0LA 1LA": ( 7,   66,  284),
    "1RB 1RC  0RC 1RA  1LD 0RB  0LD 1LA": ( 7,   50,  597),
    "1RB 1RA  1LC 0RB  1RC 0LD  1LA 1LD": ( 8,   45,  228),
    "1RB 1LA  1LC 0RA  1LD 0LC  1RA 0LA": ( 3,    5,  385),
    "1RB 0RA  1LC 1RA  1LD 0LC  1LA 0RB": ( 3,    5,  244),
    "1RB 1RC  0LD 1RA  1LB 0RD  1LA 0RC": ( 1,    2,  294),
    "1RB 0LA  0RC 1LA  1RD 1RC  1LD 1LB": ( 0,    0,  714),
    "1RB 0LC  1LD 1LC  1RD 0LA  0RA 1LB": ( 0,    0,  294),
    "1RB 1LA  1LB 0RC  1LC 1LD  0RA 0LD": ( 0,    0,  238),
    "1RB 0LA  1LB 0RC  1RD 1RC  1LA 1LD": ( 0,    0,  228),

    "1RB 0RC  1LB 1LD  0RA 0LD  1LA 1RC": (503, 158491, 17620),
    "1RB 0RA  1RC 0RB  1LD 1LC  1RA 0LC": (102,   7170, 29117),
    "1RB 1RA  0RC 0LB  0RD 0RA  1LD 0LA": (203,  28812,  5588),
}

RECURRENCE_FAST_FIXED = {
    # 2/2
    "1RB 1LB  1LA 1RA": (4, 5, 2),  # center

    # 2/3
    "1RB 2LA 2RB  1LB 1LA 1RA": (8, 39, 2),  # center, >BB
}

BLANK_FAST = {
    # 2/2
    "1RB 0RA  1LB 1LA": 8,
    "1RB 0RA  0LB 1LA": 7,
    "1RB 1LA  0LA 0LB": 6,
    "1RB 0LA  1LB 1RA": 5,
    "1RB 1RB  1LA 0LB": 5,
    "1RB ...  1LB 0RB": 4,
    "1RB 0RA  1LA ...": 4,

    # 3/2
    "1RB 1LB  1LA 1LC  1RC 0LC": 34,
    "1RB 1LC  1LB 1LA  1RC 0LC": 27,
    "1RB 1LB  1LA 1RC  1LC 0RC": 26,
    "1RB 1LB  1LA 0LC  1RC 0LC": 25,
    "1RB 0RB  1LC 1RC  0LA 1LA": 25,
    "1RB 0RB  1LC 0LC  1LA 1RA": 23,
    "1RB 0LB  1LA 1LC  1RC 0LC": 23,
    "1RB 1LB  1LA 1RC  0RB 0LC": 22,
    "1RB 1LB  0RC 1LA  1LA 0RA": 21,
    "1RB 1LA  1LA 1RC  1LC 0RC": 20,
    "1RB 1LA  1LA 1LC  1RC 0LC": 20,
    "1RB 0LC  1LB 1LA  1RC 0LC": 20,
    "1RB 0LB  1LA 1LC  0RC 0RB": 20,
    "1RB 1RC  1LC 0LB  1RA 1LA": 16,
    "1RB ...  1LC 0LC  1RC 0LB": 16,
    "1RB ...  0RC 1LB  1LA 0RB": 15,
    "1RB 1LB  0LC 0RB  1RA 1LA": 14,

    # 2/3
    "1RB 2LA 0RB  1LA 0LB 1RA": 77,
    "1RB 2RA 2RB  2LB 1LA 0RB": 29,
    "1RB 2RB 0RA  2LA 1LA 1LB": 27,
    "1RB 2RB 2RA  2LB 1LA 0RB": 24,
    "1RB 1LA 2RB  2LA 2RA 0LB": 24,
    "1RB 0RB 1LA  2LA 2RA 0LB":  4,

    # 4/2
    "1RB 0LC  1LD 0LA  1RC 1RD  1LA 0LD": 66345,
    "1RB 1RA  0RC 0RB  0RD 1RA  1LD 1LB":  2566,
    "1RB 1RA  0RC 1LA  1LC 1LD  0RB 0RD":  2510,
    "1RB 0RB  1LC 0LC  1RA 0LD  1LB 0LB":   976,
    "1RB 1LC  1LA 0RD  0RD 0RC  1LD 1LA":   711,
    "1RB 1LC  1LD 0RD  0RD 0RC  1LD 1LA":   709,
    "1RB 1LC  1RC 0RD  0RD 0RC  1LD 1LA":   704,
    "1RB 1LC  0RC 0RD  0RD 0RC  1LD 1LA":   702,
    "1RB 1LC  1LA 1RB  0RD 0RC  1LD 1LA":   534,
    "1RB 1LA  0LC 0LB  1RC 1RD  1LA 1RB":   495,
    "1RB 1LC  0RC 1RB  0RD 0RC  1LD 1LA":   455,
    "1RB 1RA  0RC 0RB  1LC 1LD  1RA 1LB":   426,
    "1RB 1RA  1LC 0RD  1LB 1LD  1RA 0RB":   319,
    # constructed from BB(3) sigma champ
    "1RB 1LC  1RC 1LD  1LA 0LB  1RD 0LD":    77,
    # constructed from BB(3) shift champ
    "1RB 1LC  1LB 0RD  1RC 0LC  1LD 1LA":    66,
    "1RB 0RA  0LB 0LC  1RD 1LC  1RA 1LB":     3,
    "1RB 1RC  0LD 1RA  1LB 0RD  1LA 0RC":     3,

    # 5/2
    "1RB 1LC  0LC 0RD  1RD 1LE  1RE 1LA  1LA 0LB": 31315,
    "1RB 1LC  1RD 1RA  1LB 0LA  1RE 0RC  1RC 0LE":  3241,
    "1RB 1RC  1LD 0RA  0RB 1RA  1LE 1LD  1RA 0RE":   725,
    "1RB 1LC  1RC 1LD  1RE 1RD  0RE 0LA  1LB 0LE":   362,
    "1RB 1RC  0RC 1LD  1LE 1RD  0LE 1RA  1LB 0LC":   277,
    "1RB 1LC  1LD 0LB  1RA 1RC  0RE 0LD  0RA ...":   182,
    "1RB 1LC  1LC 0LD  1LE 1LD  1RE 0RA  0RB 1LA":   134,
    "1RB 1LC  1LD 1RA  0LB 1LE  0LE 1LC  1RA 0RE":   127,
    "1RB 1LC  1RD 0RE  1LA 1LE  1RC 0LE  1RE 0LD":   123,
    "1RB ...  0LB 1RC  0LC 1RD  1LE 0LD  0RA 0LE":    63,
    "1RB 1RA  1RC 0RD  1LE 0RA  ... 0RB  1LB 1LE":    28,

    # 2/4
    "1RB 2RA 1RA 2RB  2LB 3LA 0RB 2LA": 190524,
    "1RB 0RA 1RA 0RB  2LB 3LA 1LA 0RA":   4078,
    "1RB 2RA 3LA 2RB  2LB 1LA 0RB 0RA":   2501,
    "1RB 2RA 1RA 2LB  2LB 3LA 0RB 0RA":   1612,
    "1RB 0RA 1RA 0RB  2LB 3LA 0RB 0RA":   1538,
    "1RB 2RB 3RB 3RA  3LB 2LA 1RA 0RB":   1065,
    "1RB 2RB 1LA 0LA  2LB 3LA 0RB 1RA":    888,
    "1RB 2RA 1RA 2LB  2LB 3LA 0RB 2LA":    759,
    "1RB 0RA 1RA 0RB  2LB 3LA 1LA 2RA":    697,
    "1RB 0RA 1RA 2RB  2LB 3LA 0RB 0RA":    673,
    "1RB 2RA 1LA 0RB  2LB 3LA 0RB 2RA":    604,
    "1RB 2RA 1RA 2RB  2LB 3LA 0RB 2RA":    562,
    "1RB 2RA 1LA 2LB  2LB 3LA 0RB 2RA":    301,
    "1RB 2RB 2RA 1LA  2LB 3LA 0RB 1RA":    281,
    "1RB 2LA 0RA 1LA  3LA 0LB 1RA 2LA":    239,
    "1RB 2LB 3LA 0RA  1LA 3RB 3LB 2RA":    224,
    "1RB 2RB 1LA 1LB  2LB 3LA 0RB 2RA":    158,
    "1RB 1LA 2RB 1LB  3LA 0LB 1RA 2RA":     91,
    "1RB 1LA 2RB 3LB  2LA 3RA 0LB 0RB":     30,
    "1RB 1LA 0LB 2RB  2LA 3RA 1LB 0LA":     27,
    "1RB 1LA 2LB 0LB  3LA 2RA 0RB 0LB":     22,
}

BLANK_SLOW = {
    # 4/2
    "1RB 1LC  1RD 1RB  0RD 0RC  1LD 1LA": 32779477,
    # TNF: "1RB 1LD  1RC 1RB  1LC 1LA  0RC 0RD"

    # 5/2 constructed form 4/2
    "1RB 1LC  1RD 0LE  0RD 0RC  1LD 1LA  1RB 1RE": 32810047,
    "1RB 1LC  0LD 1RB  0RE 0RC  1RE 1RD  1LE 1LA": 32779507,

    # 6/2 constructed from 4/2
    "1RB 1LC  1RD 1RB  0RE 1RE  1LD 1LA  0LF 1LF  0RD 0RC": 65538549,

    # 6/2 inverted from 4/2
    "1RB ...  1RC ...  1LC 1LD  1RE 1LF  1RC 1RE  0RC 0RF": 32779477,
}

UNDEFINED_FAST = {
    # 4/2 BBQ
    "... ...  ... ...  ... ...  ... ...": ( 0, 'A0'),
    "1RB ...  ... ...  ... ...  ... ...": ( 1, 'B0'),
    "1RB ...  1LC ...  ... ...  ... ...": ( 2, 'C1'),
    "1RB ...  1LC ...  ... 0LC  ... ...": ( 3, 'C0'),
    "1RB ...  1LC ...  1LA 0LC  ... ...": ( 5, 'B1'),
    "1RB ...  1LC 0LA  1LA 0LC  ... ...": ( 6, 'A1'),
    "1RB 0LD  1LC 0LA  1LA 0LC  ... ...": ( 7, 'D0'),
    "1RB 0LD  1LC 0LA  1LA 0LC  1RD ...": (11, 'D1'),

    # 4/2 BBR
    "1RB ...  1LB ...  ... ...  ... ...": ( 2, 'B1'),
    "1RB ...  1LB 1LC  ... ...  ... ...": ( 3, 'C0'),
    "1RB ...  1LB 1LC  1LA ...  ... ...": ( 6, 'C1'),
    "1RB ...  1LB 1LC  1LA 1RD  ... ...": ( 7, 'D1'),
    "1RB ...  1LB 1LC  1LA 1RD  ... 0LC": ( 9, 'D0'),
    "1RB ...  1LB 1LC  1LA 1RD  0RA 0LC": (10, 'A1'),

    # 4/2 BBP
    "1RB ...  1RC ...  1LD ...  ... ...": ( 3, 'D1'),
    "1RB ...  1RC ...  1LD ...  ... 0LC": ( 4, 'C1'),
    "1RB ...  1RC ...  1LD 1LC  ... 0LC": ( 6, 'D0'),
    "1RB ...  1RC ...  1LD 1LC  1RA 0LC": ( 7, 'A1'),
    "1RB 0RA  1RC ...  1LD 1LC  1RA 0LC": (10, 'B1'),

    # 4/2 BLB
    "1RB ...  1RC ...  ... ...  ... ...": ( 2, 'C0'),
    "1RB ...  1RC ...  1LC ...  ... ...": ( 3, 'C1'),
    "1RB ...  1RC ...  1LC 1LA  ... ...": ( 4, 'A1'),
    "1RB 1LD  1RC ...  1LC 1LA  ... ...": ( 5, 'D0'),
    "1RB 1LD  1RC ...  1LC 1LA  0RC ...": ( 8, 'B1'),
    "1RB 1LD  1RC 1RB  1LC 1LA  0RC ...": (15, 'D1'),

    # 2/4 BBH
    "... ... ... ...  ... ... ... ...": ( 0, 'A0'),
    "1RB ... ... ...  ... ... ... ...": ( 1, 'B0'),
    "1RB ... ... ...  1LB ... ... ...": ( 2, 'B1'),
    "1RB ... ... ...  1LB 1LA ... ...": ( 5, 'A1'),
    "1RB 2LA ... ...  1LB 1LA ... ...": ( 7, 'B2'),
    "1RB 2LA ... ...  1LB 1LA 3RB ...": ( 9, 'A3'),
    "1RB 2LA ... 1RA  1LB 1LA 3RB ...": (22, 'A2'),

    # some 2/4
    "1RB 3LA 1RA 0LA  2LA ... ... 3RA": (123, 'B2'),
    "1RB 3LA 1RA 0LA  2LA ... 1LB 3RA": (124, 'B1'),
    "1RB 3LA 1RA 0LA  2LA ... 0LB 3RA": (124, 'B1'),

    # 5/2 BBH
    "... ...  ... ...  ... ...  ... ...  ... ...": ( 0, 'A0'),
    "1RB ...  ... ...  ... ...  ... ...  ... ...": ( 1, 'B0'),
    "1RB ...  1RC ...  ... ...  ... ...  ... ...": ( 2, 'C0'),
    "1RB ...  1RC ...  1RD ...  ... ...  ... ...": ( 3, 'D0'),
    "1RB ...  1RC ...  1RD ...  1LA ...  ... ...": ( 4, 'A1'),
    "1RB 1LC  1RC ...  1RD ...  1LA ...  ... ...": ( 5, 'C1'),
    "1RB 1LC  1RC ...  1RD 0LE  1LA ...  ... ...": ( 6, 'E1'),
    "1RB 1LC  1RC ...  1RD 0LE  1LA ...  ... 0LA": (10, 'D1'),
    "1RB 1LC  1RC ...  1RD 0LE  1LA 1LD  ... 0LA": (16, 'B1'),
}

UNDEFINED_SLOW = {
    # 2/4 BBH last
    "1RB 2LA 1RA 1RA  1LB 1LA 3RB ...": (3932963, 'B3'),

    # 5/2 BBH last
    "1RB 1LC  1RC 1RB  1RD 0LE  1LA 1LD  ... 0LA": (47176869, 'E0'),
}

BB4_EXTENSIONS = {
    "1RB 1LB  1LA 0LC  1R_ 1LD  1RD 0RA": ('HALTED', 107),
    "1RB 1LB  1LA 0LC  0LC 1LD  1RD 0RA": ('QSIHLT', (106, 1)),
    "1RB 1LB  1LA 0LC  1LC 1LD  1RD 0RA": ('QSIHLT', (106, 1)),
    "1RB 1LB  1LA 0LC  0LB 1LD  1RD 0RA": ('LINREC', (24, 96)),
    "1RB 1LB  1LA 0LC  1RA 1LD  1RD 0RA": ('LINREC', (85, 23)),
    "1RB 1LB  1LA 0LC  1LB 1LD  1RD 0RA": ('LINREC', (89, 18)),
    "1RB 1LB  1LA 0LC  1RD 1LD  1RD 0RA": ('LINREC', (102, 7)),
    "1RB 1LB  1LA 0LC  1LD 1LD  1RD 0RA": ('LINREC', (305, 70)),
    "1RB 1LB  1LA 0LC  0RB 1LD  1RD 0RA": ('LINREC', (313, 74)),
    "1RB 1LB  1LA 0LC  1RB 1LD  1RD 0RA": ('LINREC', (341, 46)),
    "1RB 1LB  1LA 0LC  1RC 1LD  1RD 0RA": ('LINREC', (349, 50)),
    "1RB 1LB  1LA 0LC  1LA 1LD  1RD 0RA": ('LINREC', (379, 110)),
    "1RB 1LB  1LA 0LC  0RA 1LD  1RD 0RA": ('LINREC', (381, 118)),
    "1RB 1LB  1LA 0LC  0LD 1LD  1RD 0RA": ('LINREC', (397, 142)),
    "1RB 1LB  1LA 0LC  0RD 1LD  1RD 0RA": ('LINREC', (397, 142)),
    "1RB 1LB  1LA 0LC  0LA 1LD  1RD 0RA": ('LINREC', (403, 122)),
    "1RB 1LB  1LA 0LC  0RC 1LD  1RD 0RA": ('LINREC', (403, 144)),

    "1RB 0RC  1LA 1RA  1R_ 1RD  1LD 0LB": ('HALTED', 96),
    "1RB 0RC  1LA 1RA  1RC 1RD  1LD 0LB": ('QSIHLT', (95, 1)),
    "1RB 0RC  1LA 1RA  0RC 1RD  1LD 0LB": ('QSIHLT', (95, 1)),
    "1RB 0RC  1LA 1RA  0RA 1RD  1LD 0LB": ('LINREC', (0, 96)),
    "1RB 0RC  1LA 1RA  1LB 1RD  1LD 0LB": ('LINREC', (74, 23)),
    "1RB 0RC  1LA 1RA  1RA 1RD  1LD 0LB": ('LINREC', (78, 18)),
    "1RB 0RC  1LA 1RA  1LD 1RD  1LD 0LB": ('LINREC', (91, 7)),
    "1RB 0RC  1LA 1RA  1RD 1RD  1LD 0LB": ('LINREC', (294, 70)),
    "1RB 0RC  1LA 1RA  0LA 1RD  1LD 0LB": ('LINREC', (302, 74)),
    "1RB 0RC  1LA 1RA  1LA 1RD  1LD 0LB": ('LINREC', (330, 46)),
    "1RB 0RC  1LA 1RA  1LC 1RD  1LD 0LB": ('LINREC', (338, 50)),
    "1RB 0RC  1LA 1RA  1RB 1RD  1LD 0LB": ('LINREC', (368, 110)),
    "1RB 0RC  1LA 1RA  0LB 1RD  1LD 0LB": ('LINREC', (370, 118)),
    "1RB 0RC  1LA 1RA  0LD 1RD  1LD 0LB": ('LINREC', (386, 142)),
    "1RB 0RC  1LA 1RA  0RD 1RD  1LD 0LB": ('LINREC', (386, 142)),
    "1RB 0RC  1LA 1RA  0LC 1RD  1LD 0LB": ('LINREC', (392, 144)),
    "1RB 0RC  1LA 1RA  0RB 1RD  1LD 0LB": ('LINREC', (392, 122)),
}

class TuringTest(TestCase):
    def assert_normal(self, prog):
        if not prog.startswith('0'):
            self.assertEqual(
                prog,
                Program(prog).normalize())

    def assert_marks(self, marks):
        self.assertEqual(
            self.machine.marks,
            marks)

    def assert_steps(self, steps):
        self.assertEqual(
            self.machine.steps,
            steps)

    def assert_final(self, final):
        self.assertEqual(
            self.final.blanks,
            final)

    def assert_lin_recurrence(self, steps, period):
        self.assertTrue(
            self.history.verify_lin_recurrence(
                steps,
                period,
            ))

    def deny_lin_recurrence(self, steps, period):
        self.assertFalse(
            self.history.verify_lin_recurrence(
                steps,
                period,
            ))

    def verify_lin_recurrence(self, prog, steps, period):
        runtime = steps + (2 * period)

        self.run_bb(
            prog,
            xlimit=runtime,
            samples={
                steps - 1             : None,
                steps                 : None,
                steps + 1             : None,
                steps     + period    : None,
                steps + 1 + period    : None,
                steps - 1 + period    : None,
                steps     + period * 2: None,
            },
            print_prog=False,
        )

        self.assert_lin_recurrence(steps, period)
        self.assert_lin_recurrence(steps + 1, period)
        self.assert_lin_recurrence(steps, 2 * period)

        if period > 1:
            self.deny_lin_recurrence(steps, period + 1)
            self.deny_lin_recurrence(steps, period - 1)

        if steps >= 1:
            self.deny_lin_recurrence(steps - 1, period)

    def run_bb(self, prog, print_prog=True, normal=True, **opts):
        if normal:
            self.assert_normal(prog)

        if print_prog:
            print(prog)

        self.machine = run_bb(prog, **opts)
        self.history = self.machine.history
        self.final  = self.machine.final

    def _test_halting(self, prog_data):
        for prog, (marks, steps) in prog_data.items():
            self.run_bb(prog)

            self.assert_marks(marks)
            self.assert_steps(steps)

            self.assertEqual(
                steps,
                self.final.halted)

    def _test_recurrence(self, prog_data, quick,
                         qsihlt=False,
                         fixdtp=False):
        for prog, (marks, steps, period) in prog_data.items():
            print(prog)

            self.verify_lin_recurrence(
                prog,
                steps,
                period,
            )

            if not quick or period > 2000:
                continue

            self.run_bb(
                prog,
                check_rec=(
                    0
                    if steps < 256 else
                    steps),
                print_prog=False,
            )

            self.assertEqual(
                (steps, period),
                self.final.linrec)

            self.assertEqual(
                self.final.qsihlt,
                self.final.linrec if qsihlt else None)

            self.assertEqual(
                fixdtp,
                self.final.fixdtp)

            self.run_bb(
                prog,
                xlimit=steps,
                print_prog=False,
            )

            self.assert_marks(marks)

            self.assertEqual(
                steps,
                self.final.xlimit)

    def _test_undefined(self, prog_data):
        for prog, (steps, instr) in prog_data.items():
            self.run_bb(prog, normal=False)

            self.assert_steps(steps)

            self.assertEqual(
                (steps, instr),
                self.final.undfnd)

    def _test_extensions(self, prog_data):
        for prog, (status, data) in prog_data.items():
            self.run_bb(
                prog,
                check_rec=0,
            )

            self.assertEqual(
                data,
                getattr(self.final, status.lower()))

    def _test_blank(self, prog_data):
        for prog, steps in prog_data.items():
            self.run_bb(prog, check_blanks=True)

            self.assert_steps(steps)

            self.assertEqual(
                steps,
                self.final.blanks)


class Fast(TuringTest):
    def test_halting(self):
        self._test_halting(HALTING_FAST)

    @skip
    def test_recurrence(self):
        self._test_recurrence(RECURRENCE_FAST, True)

    @skip
    def test_recurrence_fixed(self):
        self._test_recurrence(
            RECURRENCE_FAST_FIXED, True,
            fixdtp=True)

    @skip
    def test_quasihalting(self):
        self._test_recurrence(
            QUASIHALTING, True,
            qsihlt=True)

    @skip
    def test_quasihalting_fixed(self):
        self._test_recurrence(
            QUASIHALTING_FIXED, True,
            qsihlt=True,
            fixdtp=True)

    def test_blank(self):
        self._test_blank(BLANK_FAST)

    def test_undefined(self):
        self._test_undefined(UNDEFINED_FAST)

    def test_bb4_extensions(self):
        self._test_extensions(BB4_EXTENSIONS)


class Slow(TuringTest):
    def test_halting(self):
        self._test_halting(HALTING_SLOW)

    @skip
    def test_quasihalting(self):
        self._test_recurrence(
            QUASIHALTING_SLOW, False,
            qsihlt=True)

    def test_blank(self):
        self._test_blank(BLANK_SLOW)

    def test_undefined(self):
        self._test_undefined(UNDEFINED_SLOW)
