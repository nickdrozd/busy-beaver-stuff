# pylint: disable = attribute-defined-outside-init, line-too-long, too-many-lines

from unittest import TestCase

from tm import run_bb
from tm.parse import tcompile, dcompile
from generate.macro import MacroConverter
from generate.graph import Graph
from generate.program import Program

HALT_FAST = {
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

    # 2/4
    "1RB 2LA 1RA 1RA  1LB 1LA 3RB 1R_": (2050, 3932964),  # BB
    "1RB 2LA 1RA 1LA  3LA 1R_ 2RB 2RA": (90, 7195),
    "1RB 2LA 1RA 1LA  3LA 1R_ 2RB 2LA": (84, 6445),
    "1RB 2LA 1RA 1LA  3LA 1R_ 2RB 3RA": (84, 6445),
    "1RB 2RB 3LA 2RA  1LA 3RB 1R_ 1LB": (60, 2351),

    # 3/3 copy of 2/4 BB
    "1RB 1LC 1R_  1LA 1LC 2RB  1RB 2LC 1RC": (2050, 3932964),

    # 5/2 BB
    "1RB 1LC  1RC 1RB  1RD 0LE  1LA 1LD  1R_ 0LA": (4098, 47176870),

    # 8/4 derived from 5/2 BB
    "1RB ... ... ...  1LC ... 1LD ...  2RE 0LF ... ...  1RG 1LD 1LF ...  3LF 1LD ... 3LD  2RG 2LH 1LD ...  1RE 1RG ... 1RB  1R_ 3LC 1RB ...": (4097, 23587667),

    # 5/5 derived from 5/2 BB
    "1RB ... ... ... ...  2LC ... ... ... ...  3RD 3LC ... 1LC 1R_  ... 1RD 1RB 1LE ...  4RD 1LE ... 1RD 1LC": (4097, 15721562),

    # Milton Green (1964)
    "1RB ...  0L_ ...": (1, 2),
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

HALT_SLOW = {
    # 3/3
    # Surprise-in-a-box
    "1RB 2LB 1LC  1LA 2RB 1RB  1R_ 2LA 0LC": (31, 2315619),
    "1RB 2RA 2RC  1LC 1R_ 1LA  1RA 2LB 1LC": (36089, 310341163),
    "1RB 1R_ 2RB  1LC 0LB 1RA  1RA 2LC 1RC": (107900, 4939345068),
    "1RB 2LA 1RA  1RC 2RB 0RC  1LA 1R_ 1LA": (1525688, 987522842126),
    "1RB 1R_ 2LC  1LC 2RB 1LB  1LA 2RC 2LA": (2950149, 4144465135614),
}

MACRO_HALT_FAST = {
    # 2/4 BB
    "1RB 2LA 1RA 1RA  1LB 1LA 3RB 1R_": {
        2: (1026, 1965975),
        3: ( 684, 1310990),
        4: ( 513,  982987),
        5: ( 410,  786595),
        6: ( 343,  655327),
    },

    # 5/2 BB
    "1RB 1LC  1RC 1RB  1RD 0LE  1LA 1LD  1R_ 0LA": {
        12: (1025, 3930266),
    },
}

QUASIHALT = {
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

    # 6/8 derived from 3/2-rec champ
    "1LB ... ... ... ... ... ... ...  2RC ... 3RD 1RC ... 4LE 5RC ...  6LF 6RD 5LF 1LB ... ... ... ...  7LF 5LF ... 6LF 2LB 1LB 3LB 0LB  0RD 1LB 3RD ... ... 3LB ... ...  ... 1LB 3RD 6RD ... 3LB 4LE ...": (5, 33, 24),

}

QUASIHALT_FIXED = {
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

    # 7/8 derived from 4/2-2819
    "1LB 2LC 1RD ... ... 3RE ... 3RF  0RD ... ... 1RD 2RD ... 3RA ...  4RE 4LC 5LG 3RD 4RD ... 6RD ...  0RD 2LC 7LG ... 5LG 6LC ... 0LC  4LC ... 3RA ... 1LB 5LB 3RD ...  7LG ... ... ... ... 1LB ... ...  6RA ... ... 1LB 5LG ... 3RA ...": (24, 944, 1),
}

QUASIHALT_SLOW = {
    # 4/2
    "1RB 1LC  1RD 1RB  0RD 0RC  1LD 1LA": (0, 32779478, 1),

    # 2/4
    "1RB 2LB 3RA 2LA  3LB 3RA 0RB 1RB": (2747, 2501552, 1),
    "1RB 2RB 1LB 1LA  1LB 3RA 3LA 2RB": (3340, 2333909, 1),

    # 7/7 from 4/2 QH
    "1RB ... ... ... ... ... ...  0LC 2LD ... ... ... 3LD ...  4RE 1RF ... ... ... ... ...  2RE 0LD 0LC ... 1RE ... ...  1RE 0LD 1RB 1LG 1RF 1LG 5LG  6LG 4LD ... ... ... 0LD 5LG  2RF 1LG 1LC ... 1RB ... ...": (0, 10925753, 1),
}

RECUR_FAST = {
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
    # "1RB 0RB 1LA  2LA 2RA 0LB": ( 3,  10, 48),
    "1RB 2LA 1RB  1LB 1LA 0RA": ( 4,   7, 46),
    # "1RB 0RA 1LB  2LA 2RB 0LA": ( 3,   6, 48),
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
    "1RB 0RC  1LD 0RA  0LD 0LB  1LA 1LB": (68, 4391,   24),
    "1RB 0LA  0RC 0RD  1LC 1LA  0RB 1RD": (70, 3957,  265),
    "1RB 0LC  0RD 1RD  1LA 1LC  1RC 0RB": (49, 3316,  208),
    "1RB 0RA  1RC 0LD  0LB 1RA  0LA 1LD": (32, 3115,  860),
    "1RB 0LB  1LA 0LC  1LB 0RD  1RC 0RB": (40, 2374,  359),
    "1RB 0LA  1LC 0RA  0LD 1RD  1LA 0RB": (45, 2110,   36),
    "1RB 0LC  1RC 0RD  1LA 1LC  1RA 0RB": (33, 1978,    8),
    "1RB 1RC  1LC 0RB  1LD 0RA  1RA 0LB": (51, 1727,  622),
    "1RB 0LC  1RD 1RA  1LA 1LD  1LC 0RA": (26, 1709,   32),
    "1RB 0LA  0RC 1RD  1LD 0RB  1LA 1RD": (22, 1709,   13),
    "1RB 0RC  1LB 0LC  0RD 0LD  1RA 0LA": (29, 1680,    5),
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

    # 2/4
    "1RB 2LA 3LA 1LA  2LB 3RA 0RA 2RB": (174, 28284, 5),
    "1RB 2LA 0LB 1RA  1LB 3LA 3RB 3RB": (98, 6697, 87),
    "1RB 2LB 0LA 1LA  2LA 3RA 1RB 0LB": (88, 5632, 13),
    "1RB 1LB 2LA 3LA  1LA 2RB 3LB 0RA": (33, 5281, 7),
    "1RB 0LA 2RB 0RB  3LB 2LA 1RA 1RA": (89, 4996, 81),
    "1RB 2RA 0LB 1LA  3LA 2RB 1LA 1RA": (77, 4702, 39),
    "1RB 0LB 1LB 1LB  2LA 3RB 1RA 0RA": (54, 4632, 92),
    "1RB 2LA 3RB 2RB  3LA 3RA 0LB 1RA": (72, 4325, 199),
    "1RB 2LB 1LA 0LB  3LA 3RA 1RB 0RA": (63, 4300, 196),
    "1RB 2LB 1LA 0RB  3LA 2RA 3LB 1RB": (115, 4111, 49),
    "1RB 0LB 1LB 2LA  2LA 0RA 3RA 2RB": (71, 4050, 280),
    "1RB 2RB 3LB 0RA  1LA 3RB 2LA 2RA": (74, 4000, 40),
    "1RB 2LB 1RA 3LA  2LA 0LA 3RB 1RA": (75, 3665, 223),
    "1RB 2RB 0LB 1LA  3LA 3RA 1LA 1LB": (76, 3439, 77),
    "1RB 2LB 3RA 1RA  3LA 0LB 1RA 0RA": (43, 3294, 240),
    "1RB 2LB 3LA 0RB  2LA 1RA 1RB 2RA": (68, 3231, 246),
    "1RB 2LB 3RA 0LB  1LA 3RA 3RB 2LA": (48, 3010, 26),
    "1RB 2LA 3RA 2LB  2LA 2RA 3RB 0LA": (64, 2991, 41),
    "1RB 2RA 1LB 2RB  2LA 2RB 3LA 0RA": (69, 2983, 77),
    "1RB 2LB 0RA 2LB  2LA 3LA 0LB 3RA": (35, 2973, 290),
    "1RB 2LB 1RA 2LA  1LA 3RB 0RA 3LB": (72, 2931, 8),
    "1RB 2LA 3RA 0LB  1LB 1LA 0RA 2LB": (75, 2898, 240),
    "1RB 2LA 3RB 1RA  3LB 2LA 0RA 0RB": (41, 2803, 80),
    "1RB 2LB 0RA 1RA  3LA 1LA 0LB 2RA": (84, 2723, 85),
    "1RB 0LB 0RA 1LA  2LA 2RA 3RA 3LB": (27, 2693, 11),
    "1RB 2LB 3LA 1LB  1LA 2RB 3RB 0RA": (74, 2618, 181),
    "1RB 0RA 0LB 2RB  3LA 3RB 0LA 2RA": (45, 2583, 291),
    "1RB 2LB 2RB 2RA  3LA 2RB 0LB 1RA": (50, 2561, 238),
    "1RB 2RA 0RB 3RA  1LB 2LA 3LA 0LB": (39, 2508, 10),
    "1RB 2LB 1RA 0RB  2LA 0LA 3RA 1LA": (56, 2468, 212),
    "1RB 2LA 0RA 3LB  0LB 2LA 3RB 1RA": (48, 2439, 222),
    "1RB 0LB 2RA 2LB  2LA 0RA 3RB 1LB": (36, 2393, 11),
    "1RB 2LA 1RB 0LB  1LA 3RA 3RB 1LB": (72, 2380, 294),
    "1RB 2RB 3LB 1LA  2LA 2RA 3LA 0RA": (49, 2295, 59),
    "1RB 0RB 1LB 1LB  1LA 2RB 3LB 0RA": (61, 2230, 95),
    "1RB 2LB 0RA 2LB  2LA 3LA 0LB 0RA": (31, 2190, 272),
    "1RB 2RA 2LB 3LA  3LA 2LB 0LA 1RB": (35, 2174, 32),
    "1RB 2LA 2LB 1LB  2LA 3RB 1RA 0LA": (51, 2161, 246),
    "1RB 2LB 3RB 2RA  1LA 0RA 0LA 2LB": (40, 2029, 128),
    "1RB 2RB 0RB 2RA  2LB 3RA 1LB 2LA": (45, 2027, 91),
    "1RB 2LA 3LB 0RB  3LB 3LA 0RA 1RB": (51, 2024, 85),
    "1RB 2LB 3RB 1RA  2LA 0RA 3RA 1LB": (49, 2011, 3),
    "1RB 0LB 0RB 2RB  3LA 3RA 1LB 2LB": (28, 1898, 5),
    "1RB 2RA 0RA 0LB  3LA 2LA 1LB 0RB": (34, 1798, 3),
    "1RB 1RA 0RB 2LB  1LB 2LA 3RB 0LA": (9, 1740, 7),
    "1RB 2LB 0RA 0LB  1LA 3RB 1LB 2RB": (34, 1582, 8),
    "1RB 2LB 1LA 3RB  3LA 2RA 3LB 2RB": (38, 1256, 8),
    "1RB 2LA 1RA 0RB  3LA 1LB 2RA 2LB": (36, 1179, 4),

    # "1RB 2LA 3LB 0RB  0LB 3LA 2RB 1RA": (31, 1470, 52),
    # "1RB 2LA 3RA 0LB  2LA 2RA 3RA 0LB": (16, 646, 156),
    # "1RB 2LA 3RA 1LB  2LA 0RA 1RA 0LA": (44, 3425, 53),
    # "1RB 2LB 0RB 2RA  3LA 3RB 0LB 3RB": (11, 28, 134),
    # "1RB 2LB 1RB 3RA  2LA 2RA 3RB 0LB": (20, 769, 188),
    # "1RB 2LB 2LA 3RB  2LA 2RA 3RA 0LB": (20, 859, 208),
    # "1RB 2LB 2RA 2LB  3LA 2RA 0LB 1RA": (23, 867, 252),
    # "1RB 2LB 3LA 0RA  2LA 2RB 0LB 0RA": (37, 1658, 4),
    # "1RB 2RB 3RB 1LA  1LB 3RA 0LB 2LA": (11, 2189, 4),
    # "1RB 2RA 3LB 2RB  3LA 0LB 0LA 1RA": (25, 2396, 68),
}

RECUR_FAST_FIXED = {
    # 2/2
    "1RB 1LB  1LA 1RA": (4, 5, 2),  # center

    # 2/3
    "1RB 2LA 2RB  1LB 1LA 1RA": (8, 39, 2),  # center, >BB

    # 2/4
    "1RB 0RB 2LB 1RA  3LA 1RA 3LB 2RB": (33, 1089, 2),
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
    "1RB 0RB ...  2LA ... 0LB":  4,

    # 4/2
    "1RB 1LC  1RD 1RB  0RD 0RC  1LD 1LA": 32779477,
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
    "1RB ...  0RC 0LA  1LC 1LD  0RB 0RD":   169,
    "1RB 1LC  1RC 1LD  1LA 0LB  1RD 0LD":    77,
    "1RB 1LC  1LB 0RD  1RC 0LC  1LD 1LA":    66,
    "1RB ...  0LC ...  ... 0RD  ... ...":     3,

    # 2/4
    "1RB 2RB 3LA 2RA  2LB 1LA 0RB 3RA": 1012664081,
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
    "1RB 1LA 0LB 2RB  2LA 3RA ... 0LA":     27,
    "1RB 1LA 2LB 0LB  3LA 2RA 0RB 0LB":     22,

    # 5/2
    "1RB 1RA  1RC ...  0RD 0RC  1LD 1LE  1RA 1LC": 32738606,
    "1RB ...  1RC 1RB  0RD 0RC  1LD 1LE  1LA 1LC": 32738619,
    "1RB 1LC  1RD 1RB  0RE 0RC  0RC ...  1LE 1LA": 32748801,
    "1RB 1RC  1LD ...  0LE 0LC  0LC 1LD  1RE 1RA": 32748815,
    "1RB ...  0LC 0LB  1RC 1RD  1LE 1RB  1LA 1LE": 32759027,
    "1RB 1LA  0LC 0LB  1RC 1RD  1RE 1RB  1LA ...": 32759041,
    "1RB 1RA  1LC ...  1RA 1LD  0RE 0RD  1LE 1LC": 32769252,
    "1RB ...  1LC 1RB  1LA 1LD  0RE 0RD  1LE 1LC": 32769266,
    "1RB ...  0RC 0RB  1LC 1LD  1RE 1LB  1RC 1RE": 32779475,
    "1RB ...  1RC 1RB  1LC 1LD  1RB 1LE  0RC 0RE": 32779477,
    "1RB 1RA  1RC ...  1LC 1LD  0RA 1LE  0RC 0RE": 32779491,
    "1RB 1RC  1LD ...  0LE 0LC  1RE 1LD  1RE 1RA": 32779492,
    "1RB 1RC  0LD ...  0LE 0LC  1LE 1LD  1RE 1RA": 32779507,
    "1RB ...  1RC 1LD  0RE 1RC  0RE 0RD  1LE 1LB": 32789702,
    "1RB ...  1LC 1RD  0LE 1LC  0LE 0LD  1RE 1RB": 32789703,
    "1RB ...  0LC 1LB  1RC 1RD  1LB 1RE  0LC 0LE": 32789705,
    "1RB ...  0LC 0LB  1RC 1RD  1LE 1RB  0LC 1LE": 32789705,
    "1RB ...  1LC 1LB  0LD 0LC  1RD 1RE  0LB 1RC": 32789715,
    "1RB ...  1RC 1RB  0RD 0RC  1LD 1LE  0RB 1LC": 32789716,
    "1RB ...  0LC 1RD  1LD 1LC  0LE 0LD  1RE 1RB": 32789718,
    "1RB ...  0LC 0LB  1RC 1RD  0LE 1RB  1LB 1LE": 32789720,
    "1RB ...  0RC 1LD  0RD 1RC  0RE 0RD  1LE 1LB": 32799939,
    "1RB ...  1LB 1LC  0RD 1LE  0RE 1RD  0RB 0RE": 32799945,
    "1RB ...  1LB 1LC  1RD 1LE  0LE 1RD  0RB 0RE": 32799960,
    "1RB 1LC  1RD 0LE  0RD 0RC  1LD 1LA  ... 1RA": 32810047,
    "1RB ...  0RC 1LD  1LD 1RC  0RE 0RD  1LE 1LB": 32810198,
    "1RB ...  0RC 0RB  1LC 1LD  0RE 1LB  1LB 1RE": 32810202,
    "1RB ...  0RC 0RB  1LC 1LD  1RE 1LB  0LD 1RE": 32810217,
    "1RB ...  0RC 1LD  1LB 1RC  0RE 0RD  1LE 1LB": 32820457,
    "1RB ...  1LB 1LC  1RD 1LE  1LE 0RC  0RB 0RE": 32830250,
    "1RB ...  1RC 1LB  1LD 1RD  0LE 0LD  1RE 0RB": 32871345,
    "1RB ...  1LB 0LC  1LD 1RC  1RE 1LE  0RB 0RE": 32871355,
    "1RB ...  1LC 1RB  1RD 1LD  0RE 0RD  1LE 0LB": 32871357,
    "1RB 1LC  1RD 0LB  0RE 0RC  0RC ...  1LE 1LA": 32891763,
    "1RB ...  1LB 1LC  0LD 1LE  1RD 0LE  0RB 0RE": 32891775,

    # constructed from 4/2 BLB
    "1RB 1LC  1RD 0LE  0RD 0RC  1LD 1LA  1RB 1RE": 32810047,
    "1RB 1LC  0LD 1RB  0RE 0RC  1RE 1RD  1LE 1LA": 32779507,

    "1RB 1LC  0LC 0RD  1RD 1LE  1RE 1LA  1LA 0LB": 31315,
    "1RB 1LC  1RD 1RA  1LB 0LA  1RE 0RC  1RC 0LE":  3241,

    # 6/2 inverted from 4/2 BLB
    "1RB ...  1RC ...  1LC 1LD  1RE 1LF  1RC 1RE  0RC 0RF": 32779477,
}

BLANK_SLOW = {
    # 2/4
    "1RB 2RA 1RA 2RB  2LB 3LA 0RB 0RA": 1367361263049,

    # 5/2 blb
    "1RB ...  0LC 0LB  0LD 1LC  1RD 0RE  1LB 1LA": 455790469746,
    "1RB ...  0RC 1RB  1LC 1LD  1RE 0RD  0LE 0RB": 455790469742,
    "1RB 1RA  0RC 0RB  1LC 1LD  1RE 1LB  ... 0RA": 348098678510,

    # 6/2 constructed from 4/2
    "1RB 1LC  1RD 1RB  0RE 1RE  1LD 1LA  0LF 1LF  0RD 0RC": 65538549,
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

    # 2/4 BLB
    "1RB ... ... ...  2LB ... ... ...": ( 2, 'B1'),
    "1RB ... ... ...  2LB 3LA ... ...": ( 4, 'B3'),
    "1RB ... ... ...  2LB 3LA ... 0RA": ( 5, 'A2'),
    "1RB ... 1RA ...  2LB 3LA ... 0RA": ( 9, 'A1'),
    "1RB 2RA 1RA ...  2LB 3LA ... 0RA": (10, 'A3'),
    "1RB 2RA 1RA 2RB  2LB 3LA ... 0RA": (11, 'B2'),

    # 2/4 former BBB
    "1RB ... ... ...  2LB 3RB ... ...": ( 3, 'B2'),
    "1RB ... ... ...  2LB 3RB 0RB ...": ( 6, 'B3'),
    "1RB ... ... ...  2LB 3RB 0RB 1RA": ( 7, 'A2'),
    "1RB ... 1LA ...  2LB 3RB 0RB 1RA": ( 8, 'A1'),
    "1RB 2RA 1LA ...  2LB 3RB 0RB 1RA": (37, 'A3'),

    # 2/4 BBB
    "1RB ... ... ...  0LB ... ... ...": ( 2, 'B1'),
    "1RB ... ... ...  0LB 2RB ... ...": ( 4, 'B2'),
    "1RB ... ... ...  0LB 2RB 3RB ...": ( 6, 'B3'),
    "1RB ... ... ...  0LB 2RB 3RB 1LA": (13, 'A1'),
    "1RB 2LA ... ...  0LB 2RB 3RB 1LA": (21, 'A3'),
    "1RB 2LA ... 1LB  0LB 2RB 3RB 1LA": (29, 'A2'),

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

    # 5/2 blank 2190942280098521917
    "1RB ...  1RC ...  1RD ...  0RE ...  ... ...": ( 4, 'E0'),
    "1RB ...  1RC ...  1RD ...  0RE ...  1LE ...": ( 6, 'E1'),
    "1RB ...  1RC ...  1RD ...  0RE ...  1LE 0LA": ( 7, 'A1'),
    "1RB 0LC  1RC ...  1RD ...  0RE ...  1LE 0LA": ( 8, 'C1'),
    "1RB 0LC  1RC ...  1RD 0RB  0RE ...  1LE 0LA": (11, 'D1'),
    "1RB 0LC  1RC ...  1RD 0RB  0RE 0RD  1LE 0LA": (28, 'B1'),

    # 5/2 BLB (?)
    "1RB ...  1RC ...  1RD ...  0RE ...  1LE 1LA": ( 7, 'A1'),
    "1RB 1LC  1RC ...  1RD ...  0RE ...  1LE 1LA": ( 8, 'C1'),
    "1RB 1LC  1RC ...  1RD 0RC  0RE ...  1LE 1LA": (19, 'B1'),
    "1RB 1LC  1RC 0RD  1RD 0RC  0RE ...  1LE 1LA": (20, 'D1'),

    # 5/2 QH xmas
    "1RB ...  1LD ...  ... ...  ... ...  ... ...": (2, 'D1'),
    "1RB ...  1LD ...  ... ...  ... 1RA  ... ...": (3, 'A1'),
    "1RB 1RC  1LD ...  ... ...  ... 1RA  ... ...": (4, 'C0'),
    "1RB 1RC  1LD ...  0RE ...  ... 1RA  ... ...": (5, 'E0'),
    "1RB 1RC  1LD ...  0RE ...  ... 1RA  1LB ...": (9, 'C1'),
    "1RB 1RC  1LD ...  0RE 0RC  ... 1RA  1LB ...": (13, 'D0'),
    "1RB 1RC  1LD ...  0RE 0RC  1LC 1RA  1LB ...": (23, 'E1'),
    "1RB 1RC  1LD ...  0RE 0RC  1LC 1RA  1LB 1RE": (27, 'B1'),
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

SPAGHETTI = {
    # Halt
    "1RB 2RA 2RC  1LC 1R_ 1LA  1RA 2LB 1LC",  # 310341163

    # Quasihalt
    "1RB 1LC  0LC 0RD  1RD 1LE  1RE 1LA  1LA 0LB",  # 31317, 3

    # Recur
    "1RB 0LC  1LD 1LC  1RD 0LA  0RA 1LB",  # 0, 294
    "1RB 1RC  0LD 1RA  1LB 0RD  1LA 0RC",  # 2, 294
}

KERNEL = {
    # Halt
    "1RB 0LC  1RC 1RD  1LA 0RB  0RE 1R_  1LC 1RA": 3,  # 134467 Uwe

    # Recur
    "1RB 0RC  1LB 1LD  0RA 0LD  1LA 1RC": 3, # 158491, 17620 Boyd
    "1RB 1RC  1LC 0LD  1RA 0LB  0RA 0RC": 3, #  14008,    24
    "1RB 0LC  0RD 0RC  1LD 0RB  1LA 0LC": 3,
    "1RB 0RC  1LD 0RA  0LD 0LB  1LA 1LB": 3,
    "1RB 0RC  0LD 1RA  0LA 0RD  1LC 1LA": 3,
    "1RB 0LC  1RD 1RA  1LA 1LD  1LC 0RA": 3,
    "1RB 1RC  1LC 0LD  0RA 1LB  1RD 0LA": 3,
    "1RB 0LC  1RD 0RA  0LB 0LA  1LC 0RA": 3,
    "1RB 1RC  0RC 1RA  1LD 0RB  0LD 1LA": 3,

    # Quasihalt
    "1RB 0LC  1RC 1LD  1RD 0RB  0LB 1LA": 3,
    "1RB 1LC  1LC 0RD  1LA 0LB  1LD 0RA": 3,
    "1RB 0RC  0RD 1RA  0LD 0LA  1LC 1LA": 3,

    "1RB 1LC  1LC 1RA  1LB 0LD  1LA 0RE  1RD 1RE": 3,  # 221032, 2
    "1RB 1LC  1RD 1RA  1LB 0LA  1RE 0RC  1RC 0LE": 3,

    # Quasihalt Fixed
    "1RB 0LC  1LD 0RC  1RA 0RB  0LD 1LA": 3,  # 1459, 1
    "1RB 1RC  1LD 0RA  0RC 1RD  1RA 0LB": 3,
}

MODULAR = {
    "1RB 1LB  1LA 1LC  1RC 0LC",  # BLB(3) | 34
    "1RB 1LB  1LA 1RC  1LC 0RC",
    "1RB 1LB  1LA 0LC  1RC 0LC",
    "1RB 0LB  1LA 1LC  1RC 0LC",
    "1RB 1LC  1LB 1LA  1RC 0LC",
    "1RB 1LA  1LA 1RC  1LC 0RC",
    "1RB 1LA  1LA 1LC  1RC 0LC",
    "1RB 0LC  1LB 1LA  1RC 0LC",

    # constructed from BB(3) sigma champ
    "1RB 1LC  1RC 1LD  1LA 0LB  1RD 0LD",
    # constructed from BB(3) shift champ
    "1RB 1LC  1LB 0RD  1RC 0LC  1LD 1LA",
}


class TuringTest(TestCase):
    def assert_normal(self, prog):
        self.assertTrue(
            Graph(prog).is_normal,
            prog)

        if prog.startswith('0'):
            return

        self.assertEqual(
            prog,
            Program(prog).normalize())

    def assert_comp(self, prog):
        if '.' in prog:
            return

        self.assertEqual(
            prog,
            dcompile(tcompile(prog)))

    def assert_connected(self, prog):
        if prog in MODULAR:
            return

        if 'A' not in prog or '...' in prog:
            return

        self.assertTrue(
            Graph(prog).is_strongly_connected,
            prog)

    def assert_simple(self, prog):
        if prog in SPAGHETTI or prog in KERNEL:
            return

        if len(prog) > 50:
            return

        self.assertTrue(
            Graph(prog).is_simple,
            prog)

    def assert_reached(self, prog):
        def dimension(prog):
            comp = tcompile(prog)
            return len(comp) * len(comp[0])

        self.assertEqual(
            dimension(prog) - len(self.reached),
            prog.count('...'),
            (prog, self.reached))

    def assert_marks(self, marks):
        self.assertEqual(
            self.machine.marks,
            marks)

    def assert_steps(self, steps):
        self.assertEqual(
            self.machine.steps,
            steps)

    def assert_lin_recurrence(self, steps, recurrence):
        self.assertEqual(
            self.history.states[steps],
            self.history.states[recurrence],
        )

        self.assertEqual(
            self.history.verify_lin_recurrence(
                steps,
                recurrence,
            ),
            (steps, recurrence - steps),
            self.prog,
        )

    def deny_lin_recurrence(self, steps, recurrence):
        states = self.history.states

        if states[steps] == states[recurrence]:
            self.assertIsNone(
                self.history.verify_lin_recurrence(
                    steps,
                    recurrence,
                ),
                self.prog,
            )

    def verify_lin_recurrence(self, prog, steps, period):
        recurrence = period + steps
        runtime    = period + recurrence

        self.run_bb(
            prog,
            step_lim = runtime,
            skip = False,
            print_prog = False,
            samples = {
                steps - 1           : None,
                steps               : None,
                steps + 1           : None,
                recurrence - 1      : None,
                recurrence          : None,
                recurrence + 1      : None,
                recurrence + period : None,
            },
        )

        self.assert_lin_recurrence(    steps,     recurrence)
        self.assert_lin_recurrence(1 + steps, 1 + recurrence)
        self.assert_lin_recurrence(steps, period + recurrence)

        if period > 1:
            self.deny_lin_recurrence(steps, 1 + recurrence)
            self.deny_lin_recurrence(steps, recurrence - 1)

        if steps >= 1:
            self.deny_lin_recurrence(steps - 1, recurrence)

    def run_bb(
            self, prog,
            print_prog = True,
            normal = True,
            reached = True,
            **opts):
        if not isinstance(prog, str):
            self.run_comp(prog, print_prog, **opts)
            return

        if normal:
            self.assert_normal(prog)

        self.assert_comp(prog)

        if print_prog:
            print(prog)

        self.machine = run_bb(prog, **opts)
        self.history = self.machine.history
        self.reached = self.machine.reached
        self.final  = self.machine.final
        self.tape = self.machine.tape

        if reached:
            self.assert_reached(prog)

        self.assert_simple(prog)
        self.assert_connected(prog)

        if '.' not in prog:
            _ = MacroConverter(prog).macro_prog(2)

    def run_comp(self, prog, print_prog = True, **opts):
        if print_prog:
            print('COMPILED')

        self.machine = run_bb(prog, **opts)
        self.history = self.machine.history
        self.final  = self.machine.final

    def _test_halt(self, prog_data):
        for prog, (marks, steps) in prog_data.items():
            self.run_bb(prog)

            self.assert_marks(marks)
            self.assert_steps(steps)

            self.assertEqual(
                steps,
                self.final.halted)

    def _test_macro_halt(self, prog_data):
        self._test_halt({
            MacroConverter(prog).macro_comp(cells): expected
            for prog, params in prog_data.items()
            for cells, expected in params.items()
        })

    def _test_recur(
            self, prog_data, quick,
            qsihlt = False,
            fixdtp = False):
        for prog, (marks, steps, period) in prog_data.items():
            self.prog = prog

            if period == 1:
                self.assertTrue(
                    Program(prog).can_spin_out)

                self.run_bb(prog)

            else:
                self.verify_lin_recurrence(
                    prog,
                    steps,
                    period,
                )

                if not quick or period > 2000:
                    if isinstance(prog, str):
                        print(prog)
                    continue

                self.run_bb(
                    prog,
                    check_rec = (
                        0
                        if steps < 256 else
                        steps
                    ),
                )

            if period > 1:
                self.assertEqual(
                    period,
                    self.final.linrec[1])
            else:
                r_steps, r_period = self.final.linrec

                self.assertEqual(r_period, period)

                self.assertIn(
                    r_steps,
                    (steps - 1, steps, steps + 1))

            self.assertEqual(
                self.final.qsihlt,
                self.final.linrec if qsihlt else None)

            self.assertEqual(
                fixdtp,
                self.final.fixdtp)

            self.run_bb(
                prog,
                step_lim = steps,
                print_prog = False,
                reached = False,
                skip = False,
            )

            self.assert_marks(marks)

            self.assertEqual(
                steps,
                self.final.xlimit)

    def _test_undefined(self, prog_data):
        for prog, (steps, instr) in prog_data.items():
            self.run_bb(prog, normal = False)

            self.assertEqual(
                (steps, instr),
                self.final.undfnd,
                f'"{prog}": {self.final.undfnd},')

            self.assert_steps(steps)

    def _test_extensions(self, prog_data):
        for prog, (status, data) in prog_data.items():
            self.run_bb(
                prog,
                check_rec = 0,
            )

            self.assertEqual(
                data,
                getattr(self.final, status.lower()))

    def _test_blank(self, prog_data):
        for prog, steps in prog_data.items():
            self.run_bb(prog, check_blanks = True)

            self.assert_steps(steps)

            self.assertEqual(
                steps,
                self.final.blanks)


class Fast(TuringTest):
    def test_halt(self):
        self._test_halt(HALT_FAST)

    def test_macro_halt(self):
        self._test_macro_halt(MACRO_HALT_FAST)

    def test_recur(self):
        self._test_recur(RECUR_FAST, True)

    def test_recur_fixed(self):
        self._test_recur(
            RECUR_FAST_FIXED, True,
            fixdtp = True)

    def test_quasihalt(self):
        self._test_recur(
            QUASIHALT, True,
            qsihlt = True)

    def test_quasihalt_fixed(self):
        self._test_recur(
            QUASIHALT_FIXED, True,
            qsihlt = True,
            fixdtp = True)

    def test_blank(self):
        self._test_blank(BLANK_FAST)

    def test_undefined(self):
        self._test_undefined(UNDEFINED_FAST)

    def test_bb4_extensions(self):
        self._test_extensions(BB4_EXTENSIONS)

    def test_spaghetti(self):
        for prog in SPAGHETTI:
            graph = Graph(prog)

            self.assertEqual(
                len(graph.reduced()),
                len(graph.states),
                prog)

            self.assertTrue(
                graph.is_dispersed or '_' in prog,
                prog)

        for prog, kernel in KERNEL.items():
            graph = Graph(prog)

            self.assertEqual(
                len(graph.reduced()),
                kernel,
                prog)

            self.assertFalse(
                graph.is_dispersed and graph.is_irreflexive,
                prog)

    def test_tape(self):
        self.run_bb(
            "1RB 2LA 1R_  1LB 1LA 0RA",
            tape = 50,
            watch_tape = True)

        print(self.machine)

        tape_copy = self.tape.copy()

        _ = tape_copy.step(0, 1)

        self.assertEqual(
            self.machine.tape.signature,
            '101[2]21')

        self.assertEqual(
            tape_copy.signature,
            '10[1]121')


class Slow(TuringTest):
    def test_halt(self):
        self._test_halt(HALT_SLOW)

    def test_quasihalt(self):
        self._test_recur(
            QUASIHALT_SLOW, False,
            qsihlt = True)

    def test_blank(self):
        self._test_blank(BLANK_SLOW)

    def test_undefined(self):
        self._test_undefined(UNDEFINED_SLOW)
