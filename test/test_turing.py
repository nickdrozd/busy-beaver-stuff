# pylint: disable = attribute-defined-outside-init, line-too-long, too-many-lines

from math import isclose
from unittest import TestCase

from tm import Machine
from tm.parse import tcompile, dcompile
from generate import Graph, Program, BlockMacro

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
    "1RB 2LA 1RA 1LA  3LA 1R_ 2RB 2RA": (  90,    7195),
    "1RB 2LA 1RA 1LA  3LA 1R_ 2RB 2LA": (  84,    6445),
    "1RB 2LA 1RA 1LA  3LA 1R_ 2RB 3RA": (  84,    6445),
    "1RB 2RB 3LA 2RA  1LA 3RB 1R_ 1LB": (  60,    2351),

    # 5/2 BB
    "1RB 1LC  1RC 1RB  1RD 0LE  1LA 1LD  1R_ 0LA": (4098, 47176870),

    # 3/3
    "1RB 2RA 2RC  1LC 1R_ 1LA  1RA 2LB 1LC": (36089, 310341163),
    "1RB 1R_ 2RB  1LC 0LB 1RA  1RA 2LC 1RC": (107900, 4939345068),

    # Copy of 2/4 BB
    "1RB 1LC 1R_  1LA 1LC 2RB  1RB 2LC 1RC": (2050, 3932964),
    # Surprise-in-a-box
    "1RB 2LB 1LC  1LA 2RB 1RB  1R_ 2LA 0LC": (31, 2315619),
    # R. Blodgett
    "1RB 1LB 2LB  1RC 1LA 0RA  1LA 2RA 1R_": (9, 57),
    # David Jefferson
    "1RB 1RA 1R_  1LC 1LC 2LA  2RA 1LB 1LA": (12, 44),

    # 8/4 derived from 5/2 BB
    '  '.join([
        "1RB ... ... ...",
        "1LC ... 1LD ...",
        "2RE 0LF ... ...",
        "1RG 1LD 1LF ...",
        "3LF 1LD ... 3LD",
        "2RG 2LH 1LD ...",
        "1RE 1RG ... 1RB",
        "1R_ 3LC 1RB ...",
    ]): (4097, 23587667),

    # 5/5 derived from 5/2 BB
    '  '.join([
        "1RB ... ... ... ...",
        "2LC ... ... ... ...",
        "3RD 3LC ... 1LC 1R_",
        "... 1RD 1RB 1LE ...",
        "4RD 1LE ... 1RD 1LC",
    ]): (4097, 15721562),

    # Milton Green (1964)
    "1RB ...  0L_ ...": (1, 2),
    "1RB 1R_  0RC 1RC  0RD 0RC  1RE 1LA  0RF 0RE  1LF 1LD": (35, 436),

    # Lynn (1971)
    "1RB 1RA  1LC 0LD  0RA 1LB  1R_ 0LE  1RC 1RB": (15, 435),
    "1RB 1RC  1LC 1LD  0RA 1LB  1RE 0LB  1R_ 1RD": (22, 292),
    "1RB 0RC  1LC 0LB  1RD 1LB  1RE 0RA  0RB 1R_": (22, 217),
    # Lynn reports 522 steps
    "1RB 0LB  1LC 1R_  0LD 0LC  1LE 0RA  0LF 0LE  1RF 1RD": (42, 521),

    # Uwe (1981)

    # Castor diligentissimus et primus et perpetuus (Castor schultis)
    "1RB 0LC  1RC 1RD  1LA 0RB  0RE 1R_  1LC 1RA": (501, 134467),

    # Castor ministerialis: the Civil Servant Beaver, who
    # cares most for his progress, but does not produce anything.
    "1RB 1RA  1RC 0RD  1LE 0RA  0R_ 0RB  1LB 1LE": (0, 52),

    # Castor scientificus: the Scientific Beaver, who does
    # not produce anything either, but with more effort and
    # less effect on his position.
    "0RB 0LA  0RC 0R_  1RD 1LE  1LA 0LD  1RC 1RE": (0, 187),

    # Castor exflippus: the Beaver Freak, who tries to
    # survive as long as possible without producing
    # anything, moving on the tape, and changing his state.
    "0RB 0LA  1RC 0R_  0LC 1RD  0LD 1RE  1LA 0LE": (0, 67),
}

HALT_SLOW = {
    # 3/3
    "1RB 2LA 1RA  1RC 2RB 0RC  1LA 1R_ 1LA": (1525688, 987522842126),
    "1RB 1R_ 2LC  1LC 2RB 1LB  1LA 2RC 2LA": (2950149, 4144465135614),
}

SPINOUT_FAST = {
    # 2/2
    "1RB 1LB  1LB 1LA": (3, 6),
    "1RB 0LB  1LB 1LA": (2, 6),

    # 3/2
    "1RB 0LB  1LA 0RC  1LC 1LA": (6, 55),  # BBB(3, 2)
    "1RB 0LB  1RC 0RC  1LC 1LA": (6, 54),
    "1RB 0LC  1LB 0RC  1LC 1LA": (5, 52),  # BB extension
    "1RB 0LC  0LC 0RC  1LC 1LA": (5, 51),
    "1RB 0LC  1LA 0RC  1RC 1RB": (5, 49),
    "1RB 0LC  0RC 0RC  1LC 1LA": (5, 48),
    "1RB 1LB  1LA 1LC  1RC 0LC": (0, 34),
    "1RB 1LC  0RC ...  1LC 0LA": (5, 27),
    "1RB 1LC  1LB 1LA  1RC 0LC": (0, 27),
    "1RB 1LB  1LA 1RC  1LC 0RC": (0, 26),

    # 2/3
    "1RB 2LB 1LA  2LB 2RA 0RA": ( 8, 59),  # BBB(2, 3)
    "1RB 0LB 1RA  1LB 2LA 2RA": ( 3, 45),
    "1RB 2LB 1RA  2LB 2LA 0RA": (10, 43),
    "1RB 2RA 2LB  2LB 2LA 0LA": ( 5, 40),
    "1RB 2RA 2RB  2LB 1LA 0RB": ( 0, 29),
    "1RB 1LB 1RA  2LB 2LA 0RA": ( 6, 23),
    "1RB 2LB ...  1LB 2LA 1RB": ( 5, 17),

    # 4/2
    "1RB 1LC  1RD 1RB  0RD 0RC  1LD 1LA": ( 0, 32779478),
    "1RB 0LC  1LD 0LA  1RC 1RD  1LA 0LD": ( 0,    66349),
    "1RB 1RA  0RC 0RB  0RD 1RA  1LD 1LB": ( 0,     2568),
    "1RB 1RA  0RC 1LA  1LC 1LD  0RB 0RD": ( 0,     2512),
    "1RB 1LC  1LC 0RD  1LA 0LB  1LD 0RA": (39,     1164),
    "1RB 1LB  1RC 0LD  0RD 0RA  1LD 0LA": (20,     1153),
    "1RB 0LB  0RC 0LC  0RD 1LC  1LD 0LA": (19,      673),
    "1RB 0LC  1LD 0RA  1RC 1RB  1LA 0LB": (31,      651),
    "1RB 0LC  0RD 1LC  0LA 1LB  1LD 0RB": (22,      536),
    "1RB 0LB  1LB 1LC  1RD 0LB  1RA 0RD": (12,      444),
    "1RB ...  0RC 0LA  1LC 1LD  0RB 0RD": ( 0,      171),

    # 2/4
    "1RB 2RB 3LA 2RA  2LB 1LA 0RB 3RA": (   0, 1012664081),
    "1RB 2LB 3RA 2LA  3LB 3RA 0RB 1RB": (2747, 2501552),
    "1RB 2RA 1RA 2RB  2LB 3LA 0RB 2LA": (   0,  190524),
    "1RB 2RB 1LA 0LB  2LB 3RB 0RB 1LA": ( 190,   32849),
    "1RB 2RB 3LA 2RA  1LB 1LA 1LB 3RB": (  62,   22464), # QH 22402
    "1RB 2RA 3LA 0LB  1LB 1LA 0RB 1RB": (  99,   16634),
    "1RB 2RB 1LA 1LA  2LB 2RA 3LB 1LA": (  62,    4067), # QH 4005
    "1RB 2RB 3LA 2RA  1LB 1LA 1LB 3RA": (  42,    3247),
    "1RB 2RB 3LA 2RA  1LB 1LA 2LB 3RA": (  42,    3057),
    "1RB 2RA 3LB 2LA  1LB 3LA 3RA 1RB": (  44,    3054),
    "1RB 2LB 3RA 0LA  1LB 2RB 2LA 1LA": (  31,    2872),
    "1RB 0RB 0LA 2LB  1LB 2LA 3RB 1RA": (  32,    1769),
    "1RB 0LA 0RB 2LB  3LB 3RA 0RA 1LA": (  36,    1525),
    "1RB 0LA 0RB 2LB  3LB 3RA 1RB 1LA": (  35,    1458),

    # 5/2
    "1RB 1RC  0LC 1RD  1LB 1LE  1RD 0RA  1LA 0LE": (19670, 193023636),
    "1RB 1RA  1RC ...  0RD 0RC  1LD 1LE  1RA 1LC": (0, 32738607),
    "1RB ...  1RC 1RB  0RD 0RC  1LD 1LE  1LA 1LC": (0, 32738620),
    "1RB 1LC  1RD 1RB  0RE 0RC  0RC ...  1LE 1LA": (0, 32748802),
    "1RB 1LC  1RD 0LE  0RD 0RC  1LD 1LA  1RB 1RE": (0, 32810048),
    "1RB 1LC  0LD 1RB  0RE 0RC  1RE 1RD  1LE 1LA": (0, 32779508),

    # 7/7 from 4/2 QH
    '  '.join([
        "1RB ... ... ... ... ... ...",
        "0LC 2LD ... ... ... 3LD ...",
        "4RE 1RF ... ... ... ... ...",
        "2RE 0LD 0LC ... 1RE ... ...",
        "1RE 0LD 1RB 1LG 1RF 1LG 5LG",
        "6LG 4LD ... ... ... 0LD 5LG",
        "2RF 1LG 1LC ... 1RB ... ...",
    ]): (1, 10925753),
}

SPINOUT_FAST_FIXED = {
    # 2/2
    "1RB 1LB  0LB 1LA": (2, 6),
    "1RB 0LB  0LB 1LA": (1, 6),

    # 2/3
    "1RB 2RA 2LB  0LB 1LA 1RA": (4, 23),

    # 4/2
    "1RB 1RC  1LC 1RD  1RA 1LD  0RD 0LB": (69, 2819),  # BBB sigma
    "1RB 0LC  1LD 0RC  1RA 0RB  0LD 1LA": (25, 1459),
    "1RB 1RC  1LD 0RA  0RC 1RD  1RA 0LB": (32,  581),

    # 2/4
    "1RB 2RA 3LA 1LB  0LB 2LA 3RA 1RB": ( 31,  2476),
    "1RB 2RA 2LA 3LB  0LB 1LA 3RB 0RA": ( 30,  1854),

    # 7/8 derived from 4/2-2819
    '  '.join([
        "1RB 2RC 1LD ... ... 3LE ... 3LF",
        "0LD ... ... 1LD 2LD ... 3LA ...",
        "4LE 4RC 5RG 3LD 4LD ... 6LD ...",
        "0LD 2RC 7RG ... 5RG 6RC ... 0RC",
        "4RC ... 3LA ... 1RB 5RB 3LD ...",
        "7RG ... ... ... ... 1RB ... ...",
        "6LA ... ... 1RB 5RG ... 3LA ...",
    ]): (24, 944),
}

SPINOUT_SLOW = {
    # 2/4
    "1RB 2RB 1LB 1LA  1LB 3RA 3LA 2RB": (3340,       2333909),
    "1RB 2RA 1RA 2RB  2LB 3LA 0RB 0RA": (   0, 1367361263049),

    # 5/2 blank
    "1RB 1RA  0RC 0RB  1LC 1LD  1RE 1LB  ... 0RA": (0, 348098678511),
    "1RB ...  0RC 1RB  1LC 1LD  1RE 0RD  0LE 0RB": (0, 455790469746),
    "1RB ...  0LC 0LB  0LD 1LC  1RD 0RE  1LB 1LA": (0, 455790469748),

    # 6/2 constructed from 4/2
    "1RB 1LC  1RD 1RB  0RE 1RE  1LD 1LA  0LF 1LF  0RD 0RC": (0, 65538552),
}

SPINOUT_SLOW_FIXED = {
    # 5/2
   "1RB 0RC  1LC 0LD  1RE 0LD  0LC 1LB  0RE 1RA": (4843, 26181502),
}

QUASIHALT = {
    # 3/2
    "1RB ...  1LB 0LC  1RC 1RB": (5, 13),
    "1RB ...  1LB 1RC  0LC 0RB": (2, 14),
    "1RB ...  1LB 1LC  1RC 0RB": (2, 13),
    "1RB ...  1LC 0RB  1LB 1RC": (2, 10),

    # 2/3
    "1RB 2LA 1RA  2LB 1LA 2RB": (16, 3),
    # "1RB ... ...  2LB 1RB 1LB": ( 1, 5),

    # 4/2
    "1RB 1RC  1RD 0LC  1LD 0LD  1LB 0RA": (2332, 3),
    "1RB 0LC  1RC 1LD  1RD 0RB  0LB 1LA": (1460, 3),  # QH 1459
    "1RB 0RC  0RD 1RA  0LD 0LA  1LC 1LA": ( 334, 2),
    "1RB 0RB  1LC 1RA  0LD 1LB  1RD 0LB": ( 119, 6),
    "1RB 1LC  1LD 0RA  1RC 0LD  0LC 1LA": ( 108, 8),
    "1RB 0LC  0RD 1RC  1LA 1RD  1LD 0RB": ( 105, 8),
    "1RB 1LA  1RC 1LD  1RD 0RC  1LB 0LA": ( 101, 8),

    # 5/2
    "1RB 1LC  1LC 1RA  1LB 0LD  1LA 0RE  1RD 1RE": (221032, 2),
    "1RB 1LC  0LC 0RD  1RD 1LE  1RE 1LA  1LA 0LB": ( 31317, 3),
    "1RB 1LC  1RD 1RA  1LB 0LA  1RE 0RC  1RC 0LE": (  3247, 3),

    # 2/4
    "1RB 2LA 1RA 1LA  2LB 3LA 2RB 2RA": (10456, 3),  # QH 10353
    "1RB 0LA 1RA 0LB  2LB 3LA 2RB 0RA": ( 2859, 3),

    # 6/8 derived from 3/2-rec champ
    '  '.join([
        "1RB ... ... ... ... ... ... ...",
        "2LC ... 3LD 1LC ... 4RE 5LC ...",
        "6RF 6LD 5RF 1RB ... ... ... ...",
        "7RF 5RF ... 6RF 2RB 1RB 3RB 0RB",
        "0LD 1RB 3LD ... ... 3RB ... ...",
        "... 1RB 3LD 6LD ... 3RB 4RE ...",
    ]): (33, 24),

}

QUASIHALT_FIXED = {
    # 3/2
    "1RB 1RC  1LC 0LB  1RA 1LA": (22, 2),  # center, >BB
    "1RB 1RC  1LC 1RA  1RA 1LA": ( 9, 2),  # center, >BB sigma

    # 2/3
    "1RB 1LA 2RA  2LA 2LB 2RB": (17, 2),

    # 2/4
    "1RB 2LB 2RA 3LA  1LA 3RA 3LB 0LB": (21485, 2),
    "1RB 2LA 1RA 1LA  0LB 3LA 2RB 3RA": ( 9698, 2),  # QH 9623
    "1RB 2LA 1RA 1LA  3LA 1LB 2RB 2RA": ( 7193, 2),  # QH 7106
    "1RB 2LA 1RA 1LA  3LA 1LB 2RB 2LA": ( 6443, 2),  # QH 6362
}

RECUR_FAST = {
    # Lin-Rado examples
    "1RB ...  0RC 1LB  1LA 0RB": (9, 10),  # total recurrence
    "1RB ...  1LB 0LC  1LA 1RA": (12,  7),  # left barrier
    "1RB ...  1LC 1RA  1LA 0LC": (12,  8),  # right barrier

    # 2/2
    "1RB 0LB  1LA 0RB": (9, 3),
    "1RB 1LA  0LA 1RA": (7, 5),
    "1RB 1LB  1LA 0RB": (7, 3),
    "1RB 0RB  1LB 1RA": (0, 9),
    "1RB 0RA  1LB 1LA": (0, 8),
    "1RB 0RA  0LB 1LA": (0, 7),
    "1RB 1LA  0LA 0LB": (0, 7),
    "1RB 1LA  1LB 0RA": (0, 6),
    "1RB 0LA  1LB 1RA": (0, 5),

    # 3/2
    "1RB 1LB  0RC 0LA  1LC 0LA": (101, 24),
    "1RB 1LA  1LC 1RC  1LA 0RB": ( 69, 16),
    "1RB 1LB  1RC 0LA  1LA 1RC": ( 65, 16),
    "1RB 0LC  1LC 1RB  1RA 1LA": ( 50, 16),
    "1RB 1LC  1LA 1RB  1RB 0LA": ( 50, 12),
    "1RB 0LC  1LC 1RB  1RB 1LA": ( 50, 12),
    "1RB 0LB  1LC 0RC  1RA 1LA": ( 38, 21),
    "1RB 1LB  1LA 1RC  0RB 0LC": ( 22,  4),
    "1RB 1LA  0RC 0RA  1LC 0LA": ( 17, 36),
    "1RB ...  1LC 0RC  1RA 0LC": ( 16,  5),
    "1RB 1LB  0RC 0RB  1LC 0LA": (  4, 38),
    "1RB 0RB  1LC 0RC  0LA 1RA": (  2, 30),
    "1RB 0LA  0RC 1LA  1LC 0RB": (  0, 92),
    "1RB 0LA  1LB 0RC  1LC 1LA": (  0, 56),
    "1RB 0LA  0RC 0RC  1LC 1LA": (  0, 48),
    "1RB 1LB  0RC 1LA  1LA 0RA": (  0, 21),
    "1RB 1LB  0RC 1RC  1LA 0LA": (  0, 15),

    # 2/3
    "1RB 0LA ...  1LB 2LA 0RB": (165, 54),
    "1RB 1LB 2LA  1LA 2RB 0RA": (101, 26),
    "1RB 2RB 1LB  1LA 2RB 0LA": ( 97, 14),
    "1RB 2LA 0RB  1LA 1RB 1RA": ( 94, 20),
    "1RB 2LA 0RB  1LA 2LB 1RA": ( 89, 26),
    "1RB 1LA 1LB  1LA 2RB 0LA": ( 80, 20),
    "1RB 2LA 0RB  1LA 2LA 1RA": ( 78, 14),
    "1RB 2LA 0RB  1LB 2LA 1RA": ( 76, 14),
    "1RB 2LA 0RB  1LA 0LB 1RA": ( 75,  4),
    "1RB 2LB 2LA  2LA 0LB 0RA": ( 63, 32),
    "1RB 0RA 2LB  2LA 2RA 0LB": ( 59, 32),
    "1RB 1LB 1LB  1LA 2RB 0LA": ( 58,  8),
    "1RB 2LA 2LB  1LA 2RA 0LB": ( 57, 60),
    "1RB 1LA 2LB  2LA 2RA 0LB": ( 57, 30),
    "1RB 2LA 0RB  1LB 1RA 1RA": ( 55, 10),
    "1RB 0RB 0LB  2LA 2RA 1LB": ( 54, 40),
    "1RB 2LA 0RB  2LA ... 1RA": ( 35,  8),
    "1RB 2LA 1RB  1LB 1LA 2RA": ( 24, 46),
    "1RB 1LA 2LB  1LA 2RA 0LB": ( 20, 48),
    "1RB 2RB 2LA  1LB 1RA 0LA": ( 14, 54),
    "1RB 2LA 1RB  1LB 1LA 0RA": (  7, 46),
    "1RB 0RA 1LB  2LA 2RB 0LA": (  6, 48),
    "1RB 0RA 2LB  2LA 0LA 1RA": (  5, 28),
    # "1RB 1RA 0RB  2LB 1LA 1LB": (  4, 23),
    "1RB 2LA 0LB  1LA 2RA 2RB": (  3, 35),
    "1RB 2LA 0RB  0LB 1LA 0RA": (  2, 57),
    "1RB 0RB 0LB  1LB 2RA 1LA": (  2, 30),
    "1RB 2LB 2LA  1LA 2RB 0RA": (  1, 35),
    "1RB 2LB 0RA  1LA 2RB 2RA": (  0, 60),
    "1RB 2LB 0RA  1LA 1RB 2RA": (  0, 48),
    "1RB 2LA 1LB  0LA 0RB 1RA": (  0, 47),

    "1RB 1LA  0RC 1RC  1LD 0RB  0LD 1LA": (586388, 104),
    "1RB 1RC  1LC 0LD  1RA 0LB  0RA 0RC": (14008,   24),
    "1RB 1LC  0RC 0RD  1LA 0LA  0LC 1RB": ( 7002,  225),
    "1RB 0RA  1RC 0LB  1LD 0RD  1RA 1LB": ( 6836,  382),
    "1RB 0LC  0RC 1RC  1LA 0RD  1LC 0LA": ( 6825,  342),
    "1RB 0LC  1RD 1LD  0LA 1LB  1LC 0RD": ( 6455,   23),
    "1RB 0LC  0RD 1RD  0LA 1LC  1LA 0RA": ( 5252,    9),
    "1RB 0RC  1LD 0RA  0LD 0LB  1LA 1LB": ( 4391,   24),
    "1RB 0LA  0RC 0RD  1LC 1LA  0RB 1RD": ( 3957,  265),
    "1RB 0LC  0RD 1RD  1LA 1LC  1RC 0RB": ( 3316,  208),
    "1RB 0RA  1RC 0LD  0LB 1RA  0LA 1LD": ( 3115,  860),
    "1RB 0LB  1LA 0LC  1LB 0RD  1RC 0RB": ( 2374,  359),
    "1RB 0LA  1LC 0RA  0LD 1RD  1LA 0RB": ( 2110,   36),
    "1RB 0LC  1RC 0RD  1LA 1LC  1RA 0RB": ( 1978,    8),
    "1RB 1RC  1LC 0RB  1LD 0RA  1RA 0LB": ( 1727,  622),
    "1RB 0LC  1RD 1RA  1LA 1LD  1LC 0RA": ( 1709,   32),
    "1RB 0LA  0RC 1RD  1LD 0RB  1LA 1RD": ( 1709,   13),
    "1RB 0RC  1LB 0LC  0RD 0LD  1RA 0LA": ( 1680,    5),
    "1RB 0LC  1RD 0RD  1LA 0RC  1LB 1RC": ( 1527,  522),
    "1RB 0LC  1RC 1RD  1LD 0RC  1LA 0RB": ( 1301,  622),
    "1RB 1LC  1RD 0RB  0LC 1LA  1RC 0RA": ( 1111,  131),
    "1RB 1RC  1LB 1LC  1RD 0LB  1RA 0RD": ( 1033,  174),
    "1RB 0LC  1RD 0RB  1LC 1LA  1RC 1RA": ( 1004,  174),
    "1RB 1LA  1RC 0RD  0LA 0RC  1RC 1LC": (  979,  144),
    "1RB 1RC  1LC 0LD  0RA 1LB  1RD 0LA": (  928,  128),
    "1RB 1LA  1RC 1LD  1RD 0RC  0LD 0LA": (  869,  404),
    "1RB 0RC  1LB 1RC  1RA 0LD  1LA 1LC": (  845,  842),
    "1RB 1RC  1LC 0RB  1RA 0LD  0LC 1LD": (  600, 1374),
    "1RB 1LA  1LC 0RA  1LD 0LC  1RB 0LA": (  497,  816),
    "1RB 0RC  0LD 1RA  0LA 0RD  1LC 1LA": (  383,  200),
    "1RB 0LA  1LC 1LD  1RD 1LB  1RA 0RD": (   79,  481),
    "1RB 0LC  0RD 0RC  1LD 0RB  1LA 0LC": (   74,  945),
    "1RB 0LC  1RD 0RA  0LB 0LA  1LC 0RA": (   67,  945),
    "1RB 1LA  1RC 0RC  1LD 0RD  0LA 1LA": (   66,  284),
    "1RB 1RC  0RC 1RA  1LD 0RB  0LD 1LA": (   50,  597),
    "1RB 1RA  1LC 0RB  1RC 0LD  1LA 1LD": (   45,  228),
    "1RB 1LA  1LC 0RA  1LD 0LC  1RA 0LA": (    5,  385),
    "1RB 0RA  1LC 1RA  1LD 0LC  1LA 0RB": (    5,  244),
    "1RB 0LC  0RC 1LD  1RD 0LA  1LB 1LA": (    0,  294),
    "1RB 0LA  0RC 1LA  1RD 1RC  1LD 1LB": (    0,  714),
    "1RB 0LC  1LD 1LC  1RD 0LA  0RA 1LB": (    0,  294),
    "1RB 1LA  1LB 0RC  1LC 1LD  0RA 0LD": (    0,  238),
    "1RB 0LA  1LB 0RC  1RD 1RC  1LA 1LD": (    0,  228),

    "1RB 2LA 3LA 1LA  2LB 3RA 0RA 2RB": (28284,   5),
    "1RB 2LA 0LB 1RA  1LB 3LA 3RB 3RB": ( 6697,  87),
    "1RB 2LB 0LA 1LA  2LA 3RA 1RB 0LB": ( 5632,  13),
    "1RB 1LB 2LA 3LA  1LA 2RB 3LB 0RA": ( 5281,   7),
    "1RB 0LA 2RB 0RB  3LB 2LA 1RA 1RA": ( 4996,  81),
    "1RB 2RA 0LB 1LA  3LA 2RB 1LA 1RA": ( 4702,  39),
    "1RB 0LB 1LB 1LB  2LA 3RB 1RA 0RA": ( 4632,  92),
    "1RB 2LA 3RB 2RB  3LA 3RA 0LB 1RA": ( 4325, 199),
    "1RB 2LB 1LA 0LB  3LA 3RA 1RB 0RA": ( 4300, 196),
    "1RB 2LB 1LA 0RB  3LA 2RA 3LB 1RB": ( 4111,  49),
    "1RB 0LB 1LB 2LA  2LA 0RA 3RA 2RB": ( 4050, 280),
    "1RB 2RB 3LB 0RA  1LA 3RB 2LA 2RA": ( 4000,  40),
    "1RB 2LB 1RA 3LA  2LA 0LA 3RB 1RA": ( 3665, 223),
    "1RB 2RB 0LB 1LA  3LA 3RA 1LA 1LB": ( 3439,  77),
    "1RB 2LB 3RA 1RA  3LA 0LB 1RA 0RA": ( 3294, 240),
    "1RB 2LB 3LA 0RB  2LA 1RA 1RB 2RA": ( 3231, 246),
    "1RB 2LB 3RA 0LB  1LA 3RA 3RB 2LA": ( 3010,  26),
    "1RB 2LA 3RA 2LB  2LA 2RA 3RB 0LA": ( 2991,  41),
    "1RB 2RA 1LB 2RB  2LA 2RB 3LA 0RA": ( 2983,  77),
    "1RB 2LB 0RA 2LB  2LA 3LA 0LB 3RA": ( 2973, 290),
    "1RB 2LB 1RA 2LA  1LA 3RB 0RA 3LB": ( 2931,   8),
    "1RB 0RA 0LB 2RB  3LA 3RB 0LA 2RA": ( 2583, 291),
    "1RB 2LA 1RB 0LB  1LA 3RA 3RB 1LB": ( 2380, 294),
    "1RB 2LB 0RA 2LB  2LA 3LA 0LB 0RA": ( 2190, 272),
}

RECUR_SLOW = {
    "1RB 0LC  1LC 0RD  1LA 1LC  1RA 0RA": ( 10796,  1082),
    "1RB 0RA  1LC 1RD  1LD 0LC  0RB 0LA": ( 10346,  3333),
    "1RB 1LC  1LC 0RB  0LD 1LA  1RA 0LC": (401911,  4032),
    "1RB 0LC  1RD 1LD  1LB 1LA  1RC 0RA": ( 81600,  4155),
    "1RB 0RC  1LD 1LC  1RD 0LB  1RA 1LA": ( 81625,  4155),
    "1RB 1RC  1LD 1RD  1LB 0RA  1LA 0LC": (124052,  4155),
    "1RB 1LB  1RC 0RD  1LA 1LD  1RA 0LC": (129077,  4155),
    "1RB 0LC  1RC 0RA  1LA 0LD  1LC 0RC": ( 13184,  4402),
    "1RB 0RC  1LC 0LD  1RA 0LB  1LB 0RB": ( 15631,  4402),
    "1RB 0LB  1RC 0RA  1LD 0RB  1LB 0LC": ( 15635,  4402),
    "1RB 0RC  1LD 0RA  1RA 0LA  1LA 0LB": ( 17827,  4402),
    "1RB 1RA  0RC 0LB  0RD 0RA  1LD 0LA": ( 28812,  5588),
    "1RB 0LB  1LC 0RD  1LD 1LB  1RB 0RA": ( 14247,  7583),
    "1RB 0RC  1LD 0RA  1RB 0LB  1LA 1LB": ( 14247,  7583),
    "1RB 0LB  1RC 0RB  0LD 1RA  0LA 1LD": ( 24689,  8109),
    "1RB 0RA  0LC 1RD  0LD 1LC  1RA 0LA": (260631,  8109),
    "1RB 1RA  1LC 0LD  1RA 0LB  0LB 0RA": (103285, 11528),
    "1RB 0RC  1LD 0RA  0RA 0LD  1LA 1LD": (103755, 11528),
    "1RB 0LC  1RC 1RB  1LA 0LD  0LC 0RB": (104276, 11528),
    "1RB 1LC  1LD 0LC  0LB 0RA  1RD 1RA": (157757, 17620),
    "1RB 0RC  1LB 1LD  0RA 0LD  1LA 1RC": (158491, 17620),
    "1RB 1LB  1RC 0LD  0LA 1RA  0LB 0RA": (336580, 25506),
    "1RB 0LC  0LD 1RD  0LA 0RD  1RA 1LA": (393831, 25506),
    "1RB 0RA  1RC 0RB  1LD 1LC  1RA 0LC": (  7170, 29117),
    "1RB 0LA  1RC 0RA  1LD 0RC  0LB 1LA": ( 57810, 88381),
    "1RB 0RC  1LD 0RB  1RA 0LC  0LA 1LC": ( 58076, 88381),
    "1RB 0LA  0RC 1RD  1LA 0LD  1LC 0RD": ( 73906, 88381),
    "1RB 0LA  0RC 0RD  1LA 0RA  0LC 1RA": ( 22269896, 2353542),
    "1RB 0LA  0RC 1LA  1RD 0RD  1LB 1RB": ( 24378294, 2575984),
    # "1RB 0LC  1RD 1LC  0LA 1LB  1LC 0RD": (309086174, 7129704),

    "1RB 2LB 0LA 1LB  3LA 0RA 3RA 2RB": (244262,      7),
    "1RB 2LB 3RB 0LB  1LA 2RA 0LB 2LB": (417770,     10),
    "1RB 0RA 1LA 2RB  3LA 3LB 1LB 0RA": (480921,     13),
    "1RB 2RA 3LA 0LA  1LB 2LA 0RB 3RA": (474330,     14),
    "1RB 1RA 1LB 2RB  1LB 3LA 3LB 0RA": (  2952,  13619),
    "1RB 2LA 1LB 3LB  1LA 0RB 3RA 1LA": (168182,  13846),
    "1RB 2LA 0RB 1LB  2LA 3RA 3RB 0LB": (309786,  22222),
    "1RB 2LA 0RA 2LB  3LA 0LA 3RA 1RB": (275583,  22222),
    "1RB 2LB 3RB 1LA  2LA 2LA 3RA 0RB": (   948,  24813),
    "1RB 2LB 0LA 1RB  3LA 2LA 3RB 1RA": (   958,  24813),
    "1RB 2LA 3RB 0LA  2LB 1RA 2RA 1LA": (356499,  26628),
    "1RB 2LA 3LB 0RA  0LA 3LB 1RA 2RB": (246162,  29304),
    "1RB 2LA 0LB 3LA  1LA 2RA 3RB 1RB": (379053,  31566),
    "1RB 2LB 3LA 1LA  1LA 2RB 0RA 3RB": (379542,  31566),
    "1RB 2LA 0LB 2RA  1LA 1RA 3RB 2LB": (484890,  33170),
    "1RB 1LA 0LB 2RB  3LA 3LB 0RA 1RA": (449052,  38622),
    "1RB 0LB 1RA 2LB  2LA 2LA 3RB 0RA": (  4618,  39151),
    "1RB 2LA 0LB 1RA  3LA 2LA 3RA 0RA": (336885,  39266),
    "1RB 2LA 0LB 1RA  3LB 2LA 3RA 0RA": (336885,  39266),
    "1RB 1LA 0RB 2LA  3LA 3RA 0LB 1RA": (358244,  62244),
    "1RB 2LB 1LB 0RA  2LA 3RB 2RB 0LA": (437759,  62244),
    "1RB 2LB 2RA 1RB  2LA 3LA 3LB 0RA": ( 22017,  66470),
    "1RB 2RA 0LB 2RB  3LA 1LB 3LA 1RA": ( 67921,  66470),
    "1RB 2LA 0LA 0RB  3LB 1LA 3RA 0LA": (522354,  69112),
    "1RB 2RB 0RB 0RB  3LA 3RA 3LB 1LB": ( 44099,  99279),
    "1RB 2RB 3RB 2LA  1LB 3RA 0LA 0RA": ( 27783, 103675),
    "1RB 1LB 0RA 2RB  1LA 2RA 3LB 3LA": (173436, 271169),
    "1RB 2LB 3RA 3RB  1LA 1RA 0LB 2LA": (173435, 271169),
    "1RB 2LA 0LB 1RA  3LA 2RA 0RA 0LB": ( 49741, 298438),
}

RECUR_FAST_FIXED = {
    # 2/2
    "1RB 1LB  1LA 1RA": (5, 2),  # center

    # 2/3
    "1RB 2LA 2RB  1LB 1LA 1RA": (39, 2),  # center, >BB

    # 2/4
    "1RB 0RB 2LB 1RA  3LA 1RA 3LB 2RB": (1089, 2),
}

BLANKERS = {
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

UNDEFINED = {
    # 4/2 blb, sb
    "1RB ...  0RC 0LA  1LC 1LD  0RB 0RD": {
        "1RB ...  ... ...  ... ...  ... ...": ( 1, 'B0'),
        "1RB ...  0RC ...  ... ...  ... ...": ( 2, 'C0'),
        "1RB ...  0RC ...  1LC ...  ... ...": ( 4, 'C1'),
        "1RB ...  0RC ...  1LC 1LD  ... ...": ( 5, 'D0'),
        "1RB ...  0RC ...  1LC 1LD  0RB ...": ( 6, 'B1'),
        "1RB ...  0RC 0LA  1LC 1LD  0RB ...": (22, 'D1'),
    },

    # 4/2 BBH
    "1RB 1LB  1LA 0LC  1R_ 1LD  1RD 0RA": {
        "1RB ...  ... ...  ... ...  ... ...": (  1, 'B0'),
        "1RB ...  1LA ...  ... ...  ... ...": (  2, 'A1'),
        "1RB 1LB  1LA ...  ... ...  ... ...": (  5, 'B1'),
        "1RB 1LB  1LA 0LC  ... ...  ... ...": (  6, 'C1'),
        "1RB 1LB  1LA 0LC  ... 1LD  ... ...": (  7, 'D0'),
        "1RB 1LB  1LA 0LC  ... 1LD  1RD ...": (  8, 'D1'),
        "1RB 1LB  1LA 0LC  ... 1LD  1RD 0RA": (106, 'C0'),
    },

    # 4/2 BBB / BLB / SB
    "1RB 1LC  1RD 1RB  0RD 0RC  1LD 1LA": {
        "1RB ...  ... ...  ... ...  ... ...": ( 1, 'B0'),
        "1RB ...  1RD ...  ... ...  ... ...": ( 2, 'D0'),
        "1RB ...  1RD ...  ... ...  1LD ...": ( 3, 'D1'),
        "1RB ...  1RD ...  ... ...  1LD 1LA": ( 4, 'A1'),
        "1RB 1LC  1RD ...  ... ...  1LD 1LA": ( 5, 'C0'),
        "1RB 1LC  1RD ...  0RD ...  1LD 1LA": ( 8, 'B1'),
        "1RB 1LC  1RD 1RB  0RD ...  1LD 1LA": (15, 'C1'),
    },

    # 4/2 sigma
    "1RB 1RC  1LC 1RD  1RA 1LD  0RD 0LB": {
        "1RB ...  ... ...  ... ...  ... ...": ( 1, 'B0'),
        "1RB ...  1LC ...  ... ...  ... ...": ( 2, 'C1'),
        "1RB ...  1LC ...  ... 1LD  ... ...": ( 3, 'D0'),
        "1RB ...  1LC ...  ... 1LD  0RD ...": ( 4, 'D1'),
        "1RB ...  1LC ...  ... 1LD  0RD 0LB": ( 6, 'C0'),
        "1RB ...  1LC ...  1RA 1LD  0RD 0LB": ( 7, 'A1'),
        "1RB 1RC  1LC ...  1RA 1LD  0RD 0LB": (15, 'B1'),
    },

    # 4/2 BBP
    "1RB 0RA  1RC 0RB  1LD 1LC  1RA 0LC": {
        "1RB ...  ... ...  ... ...  ... ...": ( 1, 'B0'),
        "1RB ...  1RC ...  ... ...  ... ...": ( 2, 'C0'),
        "1RB ...  1RC ...  1LD ...  ... ...": ( 3, 'D1'),
        "1RB ...  1RC ...  1LD ...  ... 0LC": ( 4, 'C1'),
        "1RB ...  1RC ...  1LD 1LC  ... 0LC": ( 6, 'D0'),
        "1RB ...  1RC ...  1LD 1LC  1RA 0LC": ( 7, 'A1'),
        "1RB 0RA  1RC ...  1LD 1LC  1RA 0LC": (10, 'B1'),
    },

    # 4/2 Boyd
    "1RB 0RC  1LB 1LD  0RA 0LD  1LA 1RC": {
        "1RB ...  ... ...  ... ...  ... ...": ( 1, 'B0'),
        "1RB ...  1LB ...  ... ...  ... ...": ( 2, 'B1'),
        "1RB ...  1LB 1LD  ... ...  ... ...": ( 3, 'D0'),
        "1RB ...  1LB 1LD  ... ...  1LA ...": ( 6, 'D1'),
        "1RB ...  1LB 1LD  ... ...  1LA 1RC": ( 7, 'C1'),
        "1RB ...  1LB 1LD  ... 0LD  1LA 1RC": ( 9, 'C0'),
        "1RB ...  1LB 1LD  0RA 0LD  1LA 1RC": (10, 'A1'),
    },

    # 2/4 BBH
    "1RB 2LA 1RA 1RA  1LB 1LA 3RB 1R_": {
        "1RB ... ... ...  ... ... ... ...": ( 1, 'B0'),
        "1RB ... ... ...  1LB ... ... ...": ( 2, 'B1'),
        "1RB ... ... ...  1LB 1LA ... ...": ( 5, 'A1'),
        "1RB 2LA ... ...  1LB 1LA ... ...": ( 7, 'B2'),
        "1RB 2LA ... ...  1LB 1LA 3RB ...": ( 9, 'A3'),
        "1RB 2LA ... 1RA  1LB 1LA 3RB ...": (22, 'A2'),
        "1RB 2LA 1RA 1RA  1LB 1LA 3RB ...": (3932963, 'B3'),
    },

    # 2/4 BLB
    "1RB 2RB 3LA 2RA  2LB 1LA 0RB 3RA": {
        "1RB ... ... ...  ... ... ... ...": ( 1, 'B0'),
        "1RB ... ... ...  2LB ... ... ...": ( 2, 'B1'),
        "1RB ... ... ...  2LB 1LA ... ...": ( 5, 'A1'),
        "1RB 2RB ... ...  2LB 1LA ... ...": ( 7, 'A2'),
        "1RB 2RB 3LA ...  2LB 1LA ... ...": ( 9, 'B3'),
        "1RB 2RB 3LA ...  2LB 1LA ... 3RA": (11, 'B2'),
        "1RB 2RB 3LA ...  2LB 1LA 0RB 3RA": (23, 'A3'),
    },

    # 2/4 BBB / SB 10^23
    "1RB 2LA 1RA 1LB  0LB 2RB 3RB 1LA": {
        "1RB ... ... ...  ... ... ... ...": ( 1, 'B0'),
        "1RB ... ... ...  0LB ... ... ...": ( 2, 'B1'),
        "1RB ... ... ...  0LB 2RB ... ...": ( 4, 'B2'),
        "1RB ... ... ...  0LB 2RB 3RB ...": ( 6, 'B3'),
        "1RB ... ... ...  0LB 2RB 3RB 1LA": (13, 'A1'),
        "1RB 2LA ... ...  0LB 2RB 3RB 1LA": (21, 'A3'),
        "1RB 2LA ... 1LB  0LB 2RB 3RB 1LA": (29, 'A2'),
    },

    # 5/2 BBH
    "1RB 1LC  1RC 1RB  1RD 0LE  1LA 1LD  1R_ 0LA": {
        "1RB ...  ... ...  ... ...  ... ...  ... ...": ( 1, 'B0'),
        "1RB ...  1RC ...  ... ...  ... ...  ... ...": ( 2, 'C0'),
        "1RB ...  1RC ...  1RD ...  ... ...  ... ...": ( 3, 'D0'),
        "1RB ...  1RC ...  1RD ...  1LA ...  ... ...": ( 4, 'A1'),
        "1RB 1LC  1RC ...  1RD ...  1LA ...  ... ...": ( 5, 'C1'),
        "1RB 1LC  1RC ...  1RD 0LE  1LA ...  ... ...": ( 6, 'E1'),
        "1RB 1LC  1RC ...  1RD 0LE  1LA ...  ... 0LA": (10, 'D1'),
        "1RB 1LC  1RC ...  1RD 0LE  1LA 1LD  ... 0LA": (16, 'B1'),
        "1RB 1LC  1RC 1RB  1RD 0LE  1LA 1LD  ... 0LA": (47176869, 'E0'),
    },

    # 5/2 Uwe
    "1RB 0LC  1RC 1RD  1LA 0RB  0RE 1R_  1LC 1RA": {
        "1RB ...  ... ...  ... ...  ... ...  ... ...": ( 1, 'B0'),
        "1RB ...  1RC ...  ... ...  ... ...  ... ...": ( 2, 'C0'),
        "1RB ...  1RC ...  1LA ...  ... ...  ... ...": ( 3, 'A1'),
        "1RB 0LC  1RC ...  1LA ...  ... ...  ... ...": ( 4, 'C1'),
        "1RB 0LC  1RC ...  1LA 0RB  ... ...  ... ...": (14, 'B1'),
        "1RB 0LC  1RC 1RD  1LA 0RB  ... ...  ... ...": (15, 'D0'),
        "1RB 0LC  1RC 1RD  1LA 0RB  0RE ...  ... ...": (16, 'E1'),
        "1RB 0LC  1RC 1RD  1LA 0RB  0RE ...  ... 1RA": (20, 'E0'),
        "1RB 0LC  1RC 1RD  1LA 0RB  0RE ...  1LC 1RA": (134466, 'D1'),
    },

    # 5/2 BBB / SB / BLB || 10^14006
    "1RB 1LC  0LD 0LB  0RD 0LA  0LE 1LD  1RE 1RA": {
        "1RB ...  ... ...  ... ...  ... ...  ... ...": ( 1, 'B0'),
        "1RB ...  0LD ...  ... ...  ... ...  ... ...": ( 2, 'D1'),
        "1RB ...  0LD ...  ... ...  ... 1LD  ... ...": ( 3, 'D0'),
        "1RB ...  0LD ...  ... ...  0LE 1LD  ... ...": ( 4, 'E0'),
        "1RB ...  0LD ...  ... ...  0LE 1LD  1RE ...": ( 6, 'E1'),
        "1RB ...  0LD ...  ... ...  0LE 1LD  1RE 1RA": (17, 'A1'),
        "1RB 1LC  0LD ...  ... ...  0LE 1LD  1RE 1RA": (18, 'C1'),
        "1RB 1LC  0LD ...  ... 0LA  0LE 1LD  1RE 1RA": (29, 'B1'),
        "1RB 1LC  0LD 0LB  ... 0LA  0LE 1LD  1RE 1RA": (57, 'C0'),
    },

    # 5/2 10^14006
    "1RB 1LC  0LD 0LB  0LE 0LA  0LE 1LD  1RE 1RA": {
        "1RB ...  ... ...  ... ...  ... ...  ... ...": ( 1, 'B0'),
        "1RB ...  0LD ...  ... ...  ... ...  ... ...": ( 2, 'D1'),
        "1RB ...  0LD ...  ... ...  ... 1LD  ... ...": ( 3, 'D0'),
        "1RB ...  0LD ...  ... ...  0LE 1LD  ... ...": ( 4, 'E0'),
        "1RB ...  0LD ...  ... ...  0LE 1LD  1RE ...": ( 6, 'E1'),
        "1RB ...  0LD ...  ... ...  0LE 1LD  1RE 1RA": (17, 'A1'),
        "1RB 1LC  0LD ...  ... ...  0LE 1LD  1RE 1RA": (18, 'C1'),
        "1RB 1LC  0LD ...  ... 0LA  0LE 1LD  1RE 1RA": (29, 'B1'),
        "1RB 1LC  0LD 0LB  ... 0LA  0LE 1LD  1RE 1RA": (57, 'C0'),
    },

    # 5/2 10^12978
    "1RB 1LC  0LD 0LB  1RE 0LA  0LE 1LD  1RE 1RA": {
        "1RB ...  ... ...  ... ...  ... ...  ... ...": ( 1, 'B0'),
        "1RB ...  0LD ...  ... ...  ... ...  ... ...": ( 2, 'D1'),
        "1RB ...  0LD ...  ... ...  ... 1LD  ... ...": ( 3, 'D0'),
        "1RB ...  0LD ...  ... ...  0LE 1LD  ... ...": ( 4, 'E0'),
        "1RB ...  0LD ...  ... ...  0LE 1LD  1RE ...": ( 6, 'E1'),
        "1RB ...  0LD ...  ... ...  0LE 1LD  1RE 1RA": (17, 'A1'),
        "1RB 1LC  0LD ...  ... ...  0LE 1LD  1RE 1RA": (18, 'C1'),
        "1RB 1LC  0LD ...  ... 0LA  0LE 1LD  1RE 1RA": (29, 'B1'),
        "1RB 1LC  0LD 0LB  ... 0LA  0LE 1LD  1RE 1RA": (57, 'C0'),
    },

    # 5/2 10^12978
    "1RB 1LC  0LD 0LB  1RE 0LA  0LC 1LD  1RE 1RA": {
        "1RB ...  ... ...  ... ...  ... ...  ... ...": ( 1, 'B0'),
        "1RB ...  0LD ...  ... ...  ... ...  ... ...": ( 2, 'D1'),
        "1RB ...  0LD ...  ... ...  ... 1LD  ... ...": ( 3, 'D0'),
        "1RB ...  0LD ...  ... ...  0LC 1LD  ... ...": ( 4, 'C0'),
        "1RB ...  0LD ...  1RE ...  0LC 1LD  ... ...": ( 5, 'E0'),
        "1RB ...  0LD ...  1RE ...  0LC 1LD  1RE ...": ( 6, 'E1'),
        "1RB ...  0LD ...  1RE ...  0LC 1LD  1RE 1RA": (17, 'A1'),
        "1RB 1LC  0LD ...  1RE ...  0LC 1LD  1RE 1RA": (18, 'C1'),
        "1RB 1LC  0LD ...  1RE 0LA  0LC 1LD  1RE 1RA": (29, 'B1'),
    },

    # 5/2 10^4079
    "1RB 1LC  0LD 0LB  0RE 0LA  0LE 1LD  1RE 1RA": {
        "1RB ...  ... ...  ... ...  ... ...  ... ...": ( 1, 'B0'),
        "1RB ...  0LD ...  ... ...  ... ...  ... ...": ( 2, 'D1'),
        "1RB ...  0LD ...  ... ...  ... 1LD  ... ...": ( 3, 'D0'),
        "1RB ...  0LD ...  ... ...  0LE 1LD  ... ...": ( 4, 'E0'),
        "1RB ...  0LD ...  ... ...  0LE 1LD  1RE ...": ( 6, 'E1'),
        "1RB ...  0LD ...  ... ...  0LE 1LD  1RE 1RA": (17, 'A1'),
        "1RB 1LC  0LD ...  ... ...  0LE 1LD  1RE 1RA": (18, 'C1'),
        "1RB 1LC  0LD ...  ... 0LA  0LE 1LD  1RE 1RA": (29, 'B1'),
        "1RB 1LC  0LD 0LB  ... 0LA  0LE 1LD  1RE 1RA": (57, 'C0'),
    },

    # 5/2 10^1089, complex
    "1RB 1LC  1RD 0RA  0LC 1LE  1LA 0RE  0LA 1RB": {
        "1RB ...  ... ...  ... ...  ... ...  ... ...": ( 1, 'B0'),
        "1RB ...  1RD ...  ... ...  ... ...  ... ...": ( 2, 'D0'),
        "1RB ...  1RD ...  ... ...  1LA ...  ... ...": ( 3, 'A1'),
        "1RB 1LC  1RD ...  ... ...  1LA ...  ... ...": ( 4, 'C1'),
        "1RB 1LC  1RD ...  ... 1LE  1LA ...  ... ...": ( 5, 'E0'),
        "1RB 1LC  1RD ...  ... 1LE  1LA ...  0LA ...": ( 8, 'D1'),
        "1RB 1LC  1RD ...  ... 1LE  1LA 0RE  0LA ...": ( 9, 'E1'),
        "1RB 1LC  1RD ...  ... 1LE  1LA 0RE  0LA 1RB": (10, 'B1'),
        "1RB 1LC  1RD 0RA  ... 1LE  1LA 0RE  0LA 1RB": (18, 'C0'),
    },

    # 5/2 shawn 10^502
    "1RB 1LC  1RC 0RD  0LB 0RC  0RE 1RD  1LE 1LA": {
        "1RB ...  ... ...  ... ...  ... ...  ... ...": ( 1, 'B0'),
        "1RB ...  1RC ...  ... ...  ... ...  ... ...": ( 2, 'C0'),
        "1RB ...  1RC ...  0LB ...  ... ...  ... ...": ( 3, 'B1'),
        "1RB ...  1RC 0RD  0LB ...  ... ...  ... ...": ( 4, 'D0'),
        "1RB ...  1RC 0RD  0LB ...  0RE ...  ... ...": ( 5, 'E0'),
        "1RB ...  1RC 0RD  0LB ...  0RE ...  1LE ...": ( 8, 'E1'),
        "1RB ...  1RC 0RD  0LB ...  0RE ...  1LE 1LA": (11, 'D1'),
        "1RB ...  1RC 0RD  0LB ...  0RE 1RD  1LE 1LA": (18, 'A1'),
        "1RB 1LC  1RC 0RD  0LB ...  0RE 1RD  1LE 1LA": (19, 'C1'),
    },

    # 5/2 10^83
    "1RB 0RE  0LC 0LB  1RC 1RD  1LA 0RA  1RA 1LD": {
        "1RB ...  ... ...  ... ...  ... ...  ... ...": (  1, 'B0'),
        "1RB ...  0LC ...  ... ...  ... ...  ... ...": (  2, 'C1'),
        "1RB ...  0LC ...  ... 1RD  ... ...  ... ...": (  3, 'D0'),
        "1RB ...  0LC ...  ... 1RD  1LA ...  ... ...": (  4, 'A1'),
        "1RB 0RE  0LC ...  ... 1RD  1LA ...  ... ...": (  5, 'E1'),
        "1RB 0RE  0LC ...  ... 1RD  1LA ...  ... 1LD": (  8, 'B1'),
        "1RB 0RE  0LC 0LB  ... 1RD  1LA ...  ... 1LD": ( 11, 'C0'),
        "1RB 0RE  0LC 0LB  1RC 1RD  1LA ...  ... 1LD": ( 41, 'D1'),
        "1RB 0RE  0LC 0LB  1RC 1RD  1LA 0RA  ... 1LD": (310, 'E0'),
    },

    # 5/2 QH Xmas 10^28
    "1RB 1RD  1LC 1LB  1LD 1RA  0RE 0RD  1LB 1RE": {
        "1RB ...  ... ...  ... ...  ... ...  ... ...": ( 1, 'B0'),
        "1RB ...  1LC ...  ... ...  ... ...  ... ...": ( 2, 'C1'),
        "1RB ...  1LC ...  ... 1RA  ... ...  ... ...": ( 3, 'A1'),
        "1RB 1RD  1LC ...  ... 1RA  ... ...  ... ...": ( 4, 'D0'),
        "1RB 1RD  1LC ...  ... 1RA  0RE ...  ... ...": ( 5, 'E0'),
        "1RB 1RD  1LC ...  ... 1RA  0RE ...  1LB ...": ( 9, 'D1'),
        "1RB 1RD  1LC ...  ... 1RA  0RE 0RD  1LB ...": ( 13, 'C0'),
        "1RB 1RD  1LC ...  1LD 1RA  0RE 0RD  1LB ...": ( 23, 'E1'),
        "1RB 1RD  1LC ...  1LD 1RA  0RE 0RD  1LB 1RE": ( 27, 'B1'),
    },
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
    "1RB 0LC  0RC 1LD  1RD 0LA  1LB 1LA",  # 0, 294
}

KERNEL = {
    # Halt
    "1RB 0LC  1RC 1RD  1LA 0RB  0RE 1R_  1LC 1RA": 3,  # 134467 Uwe
    "1RB 0LC  1RC 1RD  1LA 0RB  0RE ...  ... 1RA": 3,  # partial
    "1RB 0LC  1RC 1RD  1LA 0RB  0RE ...  1LC 1RA": 3,  # partial

    "1RB 0LD  1RC 0RF  1LC 1LA  0LE 1R_  1LA 0RB  0RC 0RE": 3,

    # Spinout
    "1RB 1RC  0LC 1RD  1LB 1LE  1RD 0RA  1LA 0LE": 3,
    "1RB 0RC  1LC 0LD  1RE 0LD  0LC 1LB  0RE 1RA": 3,
    "1RB 1LC  1RD 0RA  0LC 1LE  1LA 0RE  0LA 1RB": 3,  # 10^1089
    "1RB 1LC  1RD ...  ... 1LE  1LA 0RE  0LA 1RB": 3,  # partial
    "1RB 1LC  1RD 0RA  ... 1LE  1LA 0RE  0LA 1RB": 3,  # partial
    "1RB 0LC  1RC 0LA  1LD 0RB  0RE 0RD  1LE 0LA": 3,  # 10^18

    # Recur
    "1RB 0RC  1LB 1LD  0RA 0LD  1LA 1RC": 3, # 158491, 17620 Boyd
    "1RB 1LC  1LD 0LC  0LB 0RA  1RD 1RA": 3, # 157757, 17620 iso
    "1RB 0LC  1RD 1LD  1LB 1LA  1RC 0RA": 3,
    "1RB 0RC  1LD 1LC  1RD 0LB  1RA 1LA": 3,
    "1RB 1LB  1RC 0RD  1LA 1LD  1RA 0LC": 3,
    "1RB 1RC  1LD 1RD  1LB 0RA  1LA 0LC": 3,
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

CANT_BLANK_FALSE_NEGATIVES = {
    "1RB 1LB  1LA 0RB",

    "1RB ...  1LC 0RB  1LB 1RC",
    "1RB 1LB  0RC 1RC  1LA 0LA",

    "1RB 2LA 0RB  0LB 1LA 0RA",
    "1RB 2LA 1LB  0LA 0RB 1RA",

    "1RB 0LA  1LC 1LD  1RD 1LB  1RA 0RD",
    "1RB 0LC  1RD 1LD  0LA 1LB  1LC 0RD",
    "1RB 0RA  1RC 0RB  1LD 1LC  1RA 0LC",
    "1RB 1RA  0RC 0LB  0RD 0RA  1LD 0LA",

    "1RB 0LA 1RA 0LB  2LB 3LA 2RB 0RA",
    "1RB 0LA 2RB 0RB  3LB 2LA 1RA 1RA",
    "1RB 0RA 0LB 2RB  3LA 3RB 0LA 2RA",
    "1RB 0RA 1LA 2RB  3LA 3LB 1LB 0RA",
    "1RB 2LA 0LA 0RB  3LB 1LA 3RA 0LA",
    "1RB 2LA 0RA 2LB  3LA 0LA 3RA 1RB",
    "1RB 2LA 0RB 1LB  2LA 3RA 3RB 0LB",
    "1RB 2LA 1LB 3LB  1LA 0RB 3RA 1LA",
    "1RB 2LB 0RA 2LB  2LA 3LA 0LB 0RA",
    "1RB 2LB 0RA 2LB  2LA 3LA 0LB 3RA",
    "1RB 2LB 3RA 0LA  1LB 2RB 2LA 1LA",
    "1RB 2LB 3RA 1RA  3LA 0LB 1RA 0RA",
    "1RB 2LB 3RA 2LA  3LB 3RA 0RB 1RB",
    "1RB 2RA 1LA 2LB  2LB 3RB 0RB 1RA",  # 10^16 SO
    "1RB 2RB 1LA 0LB  2LB 3RB 0RB 1LA",

    "1RB 1R_ 2RB  1LC 0LB 1RA  1RA 2LC 1RC",

    "1RB 0RC  1LC 0LD  1RE 0LD  0LC 1LB  0RE 1RA",
}

CANT_SPIN_OUT_FALSE_NEGATIVES = {
    "1RB 0RB 0LB  1LB 2RA 1LA",
    "1RB 2LA 0RB  0LB 1LA 0RA",

    "1RB 0LC  1RD 0RB  1LC 1LA  1RC 1RA",
    "1RB 1LC  1RD 0RB  0LC 1LA  1RC 0RA",
    "1RB 1RA  0RC 0LB  0RD 0RA  1LD 0LA",
    "1RB 1RC  1LB 1LC  1RD 0LB  1RA 0RD",

    "1RB 0LA 1RA 0LB  2LB 3LA 2RB 0RA",
    "1RB 0LA 2RB 0RB  3LB 2LA 1RA 1RA",
    "1RB 1RA 1LB 2RB  1LB 3LA 3LB 0RA",
    "1RB 2LA 0LA 0RB  3LB 1LA 3RA 0LA",
    "1RB 2LA 0LB 1RA  1LB 3LA 3RB 3RB",
    "1RB 2LA 0LB 1RA  3LB 2LA 3RA 0RA",

    "1RB 3LA 1LA 1RA  2LB 2RA 0RB 3RB",  # QH 77, xmas
    "1RB 2LA 2RB 1LA  3LB 3RA 2RB 0RB",  # QH 14, xmas
}

DO_HALT = {
    "1RB 0LD  1RC 0RF  1LC 1LA  0LE 1R_  1LF 0RB  0RC 0RE",  # 10^^15
    "1RB 0LA  1LC 1LF  0LD 0LC  0LE 0LB  1RE 0RA  1R_ 1LD",  # 10^^5
    "1RB 1R_  0LC 0LD  1LD 1LC  1RE 1LB  1RF 1RD  0LD 0RA",  # 10^1292913985
    "1RB 1R_  1RC 1RA  1RD 0RB  1LE 0RC  0LF 0LD  0LB 1LA",  # 10^197282
    "1RB 1RC  1LC 0RF  1RA 0LD  0LC 0LE  1LD 0RA  1RE 1R_",  # 10^78913
    "1RB 1LE  1RC 1RF  1LD 0RB  1RE 0LC  1LA 0RD  1R_ 1RC",  # 10^36534
    "1RB 0LD  1RC 0RF  1LC 1LA  0LE 1R_  1LA 0RB  0RC 0RE",  # 10^21132
    "1RB 0LE  1LC 0RA  1LD 0RC  1LE 0LF  1LA 1LC  1LE 1R_",  # 10^2879
    "1RB 0RF  0LB 1LC  1LD 0RC  1LE 1R_  1LF 0LD  1RA 0LE",  # 10^1762
    "1RB 0LF  0RC 0RD  1LD 1RE  0LE 0LD  0RA 1RC  1LA 1R_",  # 10^1730

    "1RB 2LA 1RA 2LB 2LA  0LA 2RB 3RB 4RA 1R_",  # 10^704
    "1RB 2LA 4RA 2LB 2LA  0LA 2RB 3RB 4RA 1R_",  # 10^211
    "1RB 2LA 4RA 2LB 2LA  0LA 2RB 3RB 1RA 1R_",  # 10^211
    "1RB 2LA 1RA 2LB 3RA  0LA 2RB 4RB 1R_ 2RA",  # 10^176
    "1RB 2LA 1RA 2LB 2RA  0LA 3RB 3RB 4RA 1R_",  # 10^113
    "1RB 2LA 1RA 2LB 3RA  0LA 2RB 4RB 1R_ 2LA",  # 10^101
    "1RB 2LA 4RA 1LB 2LA  0LA 2RB 3RB 2RA 1R_",  # 10^61
    "1RB 2LA 1RA 2RB 2LB  0LA 3RA 4RB 1R_ 1RA",  # 10^60
    "1RB 2LA 1RA 2LB 2RA  0LA 2RB 3RB 4RA 1R_",  # 10^52
}

DONT_BLANK = {
    "1RB 2LA 1LA  2LA 2RB 0RA",  # wolfram
}

DO_BLANK = {
    "1RB 0LB  1RC 0RC  0RD 1LA  1LE 1RD  0LC 0RE",  # 10^26
    "1RB 1LE  0RC 0RD  1LC 1LA  0RB 1RD  0LD 0RE",  # 10^30
    "1RB 1RA  1LC 0RB  1LE 0LD  1RA 1RE  1LB 0RA",  # 10^31
    "1RB 0RB  0RC 0RB  0RD 0RE  1LD 0LE  1LA 1RE",  # 10^31
    "1RB 1LE  0RC 0RB  0RD 0RE  1LD 0LA  1LB 1RE",  # 10^37
    "1RB 1LA  1LC 1RC  0LD 0LC  1RE 0LA  1RE 0RA",  # 10^38
    "1RB 1RA  1LB 0LC  1RC 1LD  0RE 0RA  0LB 0RD",  # 10^48
    "1RB 0RE  0RC 0RB  1LC 0LD  1RD 1LA  1LB 1RE",  # 10^49
    "1RB 0RE  0RC 0RB  0RD 0RE  1LD 0LA  0RB 1RE",  # 10^66
    "1RB 1RA  1LB 0LC  1RC 1LD  0RE 0RA  0RB 0RD",  # 10^137
    "1RB 1LA  0LC 0LB  0LD 1LC  1RE 0LA  1RE 0RA",  # 10^183
    "1RB 0LC  0LD 0LB  1RA 1LA  1RE 1LD  1RE 1RA",  # 10^315
    "1RB 1LC  0RD 0RD  0LB 0RC  0RE 1RD  1LE 1LA",  # 10^502
    "1RB 1LC  0LD 0LB  0RE 0LA  0LE 1LD  1RE 1RA",  # 10^4079
    "1RB 1LE  0LC 0LB  0LD 1LC  1RD 1RA  0RD 0LA",  # 10^4079 TNF
    "1RB 1LC  0LD 0LB  1RE 0LA  0LC 1LD  1RE 1RA",  # 10^12978
    "1RB 1LE  0LC 0LB  0LE 1LC  1RD 1RA  1RD 0LA",  # 10^12978 TNF
    "1RB 1LC  0LD 0LB  1RE 0LA  0LE 1LD  1RE 1RA",  # 10^12978
    "1RB 1LE  0LC 0LB  0LD 1LC  1RD 1RA  1RD 0LA",  # 10^12978 TNF
    "1RB 1LC  0LD 0LB  0LE 0LA  0LE 1LD  1RE 1RA",  # 10^14006
    "1RB 1LE  0LC 0LB  0LD 1LC  1RD 1RA  0LD 0LA",  # 10^14006 TNF
    "1RB 1LC  0LD 0LB  0RD 0LA  0LE 1LD  1RE 1RA",  # 10^14006
    "1RB 1LE  0LC 0LB  0LD 1LC  1RD 1RA  0RC 0LA",  # 10^14006 TNF
}

DO_SPIN_OUT = {
    "1RB 2RA 1LA 2LB  2LB 3RB 0RB 1RA",  # 10^16
    "1RB 2LA 1RA 1LB  0LB 2RB 3RB 1LA",  # 10^23

    "1RB 0LB 1LA  2LC 2LB 2LB  2RC 2RA 0LC",  # 10^62

    "1RB 0LC  1RC 0LA  1LD 0RB  0RE 0RD  1LE 0LA",  # 10^18
    "1RB 1LE  0RC 0RD  1LC 1LA  0RB 1RD  0LD 0RE",  # 10^30
    "1RB 0RB  0RC 0RB  0RD 0RE  1LD 0LE  1LA 1RE",  # 10^31
    "1RB 1LE  0RC 0RB  0RD 0RE  1LD 0LA  1LB 1RE",  # 10^37
    "1RB 1LA  1LC 1RC  0LD 0LC  1RE 0LA  1RE 0RA",  # 10^38
    "1RB 1RA  1LB 0LC  1RC 1LD  0RE 0RA  0LB 0RD",  # 10^48
    "1RB 0RE  0RC 0RB  1LC 0LD  1RD 1LA  1LB 1RE",  # 10^49
    "1RB 0RE  0RC 0RB  0RD 0RE  1LD 0LA  0RB 1RE",  # 10^66
    "1RB 0RE  0LC 0LB  1RC 1RD  1LA 0RA  1RA 1LD",  # 10^83
    "1RB 1RA  1LB 0LC  1RC 1LD  0RE 0RA  0RB 0RD",  # 10^137
    "1RB 1LA  0LC 0LB  0LD 1LC  1RE 0LA  1RE 0RA",  # 10^183
    "1RB 0LC  0LD 0LB  1RA 1LA  1RE 1LD  1RE 1RA",  # 10^315
    "1RB 1LC  1RC 0RD  0LB 0RC  0RE 1RD  1LE 1LA",  # 10^502
    "1RB 1LC  0RD 0RD  0LB 0RC  0RE 1RD  1LE 1LA",  # 10^502
    "1RB 1LC  1RD 0RA  0LC 1LE  1LA 0RE  0LA 1RB",  # 10^1089
    "1RB 1LC  0LD 0LB  0RE 0LA  0LE 1LD  1RE 1RA",  # 10^4079
    "1RB 1LE  0LC 0LB  0LD 1LC  1RD 1RA  0RD 0LA",  # 10^4079 TNF
    "1RB 1LC  0LD 0LB  1RE 0LA  0LC 1LD  1RE 1RA",  # 10^12978
    "1RB 1LE  0LC 0LB  0LE 1LC  1RD 1RA  1RD 0LA",  # 10^12978 TNF
    "1RB 1LC  0LD 0LB  1RE 0LA  0LE 1LD  1RE 1RA",  # 10^12978
    "1RB 1LE  0LC 0LB  0LD 1LC  1RD 1RA  1RD 0LA",  # 10^12978 TNF
    "1RB 1LC  0LD 0LB  0LE 0LA  0LE 1LD  1RE 1RA",  # 10^14006
    "1RB 1LE  0LC 0LB  0LD 1LC  1RD 1RA  0LD 0LA",  # 10^14006 TNF
    "1RB 1LC  0LD 0LB  0RD 0LA  0LE 1LD  1RE 1RA",  # 10^14006
    "1RB 1LE  0LC 0LB  0LD 1LC  1RD 1RA  0RC 0LA",  # 10^14006 TNF
}

DONT_SPIN_OUT = {
    "1RB 2LA 1LA  2LA 2RB 0RA",  # wolfram

    "1RB 0LB  1RC 0RC  0RD 1LA  1LE 1RD  0LC 0RE",  # 10^26
    "1RB 1RD  1LC 1LB  1LD 1RA  0RE 0RD  1LB 1RE",  # 10^28, xmas
    "1RB 1RA  1LC 0RB  1LE 0LD  1RA 1RE  1LB 0RA",  # 10^31
    "1RB 1LE  0RC 1LD  1RD 0RD  1RE 1RC  0LA 1LB",  # 10^46
}

MACRO_FAST = {
    # 2/4
    "1RB 2LA 1RA 1RA  1LB 1LA 3RB 1R_": (
        3932964,
        {(5, 3), (4, 4), (5, 4)},
    ),

    # 4/2
    "1RB 1LC  1RD 1RB  0RD 0RC  1LD 1LA": (32779478, {(5, 4)}),

    # 5/2
    "1RB 1LC  1RC 1RB  1RD 0LE  1LA 1LD  1R_ 0LA": (47176870, {(5, 4)}),

    # 3/3
    "1RB 2RA 2RC  1LC 1R_ 1LA  1RA 2LB 1LC": (310341163, ()),
}

MACRO_SLOW = {
    # 2/4
    "1RB 2RA 1RA 2RB  2LB 3LA 0RB 0RA": (
        1367361263049,
        {(2, 2), (2, 3), (2, 4)}),

    # 3/3
    "1RB 1R_ 2RB  1LC 0LB 1RA  1RA 2LC 1RC": (4939345068, ()),
    "1RB 2LA 1RA  1RC 2RB 0RC  1LA 1R_ 1LA": (987522842126, ()),
    "1RB 1R_ 2LC  1LC 2RB 1LB  1LA 2RC 2LA": (4144465135614, ()),
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

        if len(prog) > 70:
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

    def assert_could_halt(self, prog):
        self.assertFalse(
            Program(prog).cant_halt,
            f'halt false positive: {prog}')

    def assert_cant_halt(self, prog):
        self.assertTrue(
            Program(prog).cant_halt,
            f'halt false negative: "{prog}"')

    def assert_could_blank(self, prog):
        self.assertFalse(
            Program(prog).cant_blank,
            f'blank false positive: "{prog}"')

    def assert_cant_blank(self, prog):
        try:
            self.assertTrue(
                Program(prog).cant_blank)
        except AssertionError:
            if len(prog) > 70:
                return

            self.assertIn(
                prog,
                CANT_BLANK_FALSE_NEGATIVES,
                f'blank false negative: "{prog}"')

    def assert_could_spin_out(self, prog):
        self.assertFalse(
            Program(prog).cant_spin_out,
            f'spin out false positive: "{prog}"')

    def assert_cant_spin_out(self, prog):
        try:
            self.assertTrue(
                Program(prog).cant_spin_out)
        except AssertionError:
            self.assertIn(
                prog,
                CANT_SPIN_OUT_FALSE_NEGATIVES,
                f'spin out false negative: "{prog}"')

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
        if prog == "1RB 0RA 1LB  2LA 2RB 0LA":  # in-place???
            return

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

        assert period > 1

        self.deny_lin_recurrence(steps, 1 + recurrence)
        self.deny_lin_recurrence(steps, recurrence - 1)

        if steps >= 1:
            self.deny_lin_recurrence(steps - 1, recurrence)

    def run_bb(
            self, prog,
            print_prog = True,
            normal = True,
            **opts):
        if normal:
            self.assert_normal(prog)

        self.assert_comp(prog)

        if print_prog:
            print(prog)

        self.machine = Machine(prog).run(**opts)
        self.history = self.machine.history
        self.reached = self.machine.reached
        self.final  = self.machine.final
        self.tape = self.machine.tape

        self.assert_reached(prog)

        self.assert_simple(prog)
        self.assert_connected(prog)

        if len(prog) < 70:
            _ = BlockMacro(prog, 2).fully_specified

    def _test_simple_terminate(self, prog_data, fixed: bool):
        for prog, (marks, steps) in prog_data.items():
            self.run_bb(
                prog,
                check_blanks = (
                    marks != 0
                    and prog[0] == 0
                ),
            )

            self.assert_marks(marks)
            self.assert_steps(steps)

            self.assertEqual(
                steps,
                getattr(
                    self.final,
                    'halted' if '_' in prog else 'spnout'))

            (
                self.assert_cant_blank
                if marks != 0 else
                self.assert_could_blank
            )(prog)

            if '_' in prog:
                self.assert_could_halt(prog)
                self.assert_cant_spin_out(prog)

            else:
                self.assert_could_spin_out(prog)
                self.assert_cant_halt(prog)

                (self.assertTrue if fixed else self.assertFalse)(
                    self.final.fixdtp)

                self.assertTrue(
                    (graph := Graph(prog)).is_zero_reflexive
                    and not graph.is_irreflexive
                )

    def _test_halt(self, prog_data):
        self._test_simple_terminate(prog_data, fixed = True)

    def _test_spinout(self, prog_data, fixed: bool):
        self._test_simple_terminate(prog_data, fixed = fixed)

    def _test_recur(
            self, prog_data, quick,
            qsihlt = False,
            fixdtp = False):
        for prog, (steps, period) in prog_data.items():
            self.prog = prog

            self.assertGreater(period, 1)

            self.assert_cant_halt(prog)
            self.assert_cant_spin_out(prog)

            if prog not in BLANKERS:
                self.assert_cant_blank(prog)

            self.verify_lin_recurrence(
                prog,
                steps,
                period,
            )

            if not quick or period > 2000:
                print(prog)
                continue

            self.run_bb(
                prog,
                check_rec = (
                    0
                    if steps < 256 else
                    steps
                ),
                check_blanks = prog not in BLANKERS,
            )

            self.assertEqual(
                period,
                self.final.linrec[1])

            self.assertEqual(
                self.final.qsihlt,
                self.final.linrec if qsihlt else None)

            self.assertEqual(
                fixdtp,
                self.final.fixdtp)

    def _test_macro(self, prog_data, quick: bool):
        low = 1 if quick else 2

        for prog, (steps, exceptions) in prog_data.items():
            for wraps in range(low, 5):
                for cells in range(low, 6):
                    if (cells, wraps) in exceptions:
                        continue

                    macro = prog

                    for _ in range(wraps):
                        macro = BlockMacro(macro, cells)

                    print(macro)

                    result = getattr(
                        Machine(macro).run().final,
                        'halted' if '_' in prog else 'spnout')

                    self.assertTrue(
                        isclose(
                            result,
                            steps / (cells ** wraps),
                            rel_tol = .001,
                        )
                    )

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

            self.assert_could_blank(prog)


class Fast(TuringTest):
    def test_halt(self):
        for prog in DO_HALT | set(HALT_SLOW):
            self.assert_could_halt(prog)

        self._test_halt(HALT_FAST)

    def test_spinout(self):
        for prog in DO_SPIN_OUT | set(SPINOUT_SLOW):
            self.assert_simple(prog)
            self.assert_could_spin_out(prog)

        for prog in DONT_SPIN_OUT:
            self.assert_cant_spin_out(prog)

        self._test_spinout(SPINOUT_FAST, fixed = False)
        self._test_spinout(SPINOUT_FAST_FIXED, fixed = True)

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
        for prog in DONT_BLANK:
            self.assert_cant_blank(prog)

        for prog in DO_BLANK:
            self.assert_simple(prog)
            self.assert_could_blank(prog)

        self._test_blank(BLANKERS)

    def test_false_negatives(self):
        for prog in CANT_BLANK_FALSE_NEGATIVES:
            self.assertNotIn(prog, BLANKERS)
            self.assert_could_blank(prog)

        for prog in CANT_SPIN_OUT_FALSE_NEGATIVES:
            self.assertNotIn(
                prog,
                (set(SPINOUT_FAST)
                 | set(SPINOUT_SLOW)
                 | set(SPINOUT_FAST_FIXED)))
            self.assert_could_spin_out(prog)

    def test_bb4_extensions(self):
        self._test_extensions(BB4_EXTENSIONS)

    def test_mother_of_giants(self):
        mother = "1RB 1LE  0LC 0LB  0LD 1LC  1RD 1RA  ... 0LA"

        for prog in Program(mother).branch('E0'):
            self.assert_could_blank(prog)
            self.assert_could_spin_out(prog)

    def test_macro(self):
        self._test_macro(MACRO_FAST, quick = True)

    def test_undefined(self):
        for prog, sequence in UNDEFINED.items():
            self.assertEqual(
                sequence,
                {
                    partial: (step, slot)
                    for partial, step, slot in
                    Program(prog).instruction_sequence
                },
            )

            for partial, (step, slot) in sequence.items():
                self.run_bb(partial, normal = False)

                self.assertEqual(
                    (step, slot),
                    self.final.undfnd)

    def test_spaghetti(self):
        for prog in SPAGHETTI:
            graph = Graph(prog)

            self.assertEqual(
                len(graph.reduced),
                len(graph.states),
                prog)

            self.assertTrue(
                graph.is_dispersed or '_' in prog,
                prog)

        for prog, kernel in KERNEL.items():
            graph = Graph(prog)

            self.assertEqual(
                len(graph.reduced),
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

        self.assertEqual(
            self.machine.tape.signature,
            '101[2]21')

        print(self.machine)

        copy_1 = self.tape.copy()
        copy_2 = self.tape.copy()

        _ = copy_1.step(0, 2)
        _ = copy_2.step(1, 1)

        self.assertEqual(
            self.machine.tape.signature,
            '101[2]21')

        self.assertEqual(
            copy_1.signature,
            '10[1]21')

        self.assertEqual(
            copy_2.signature,
            '101[2]1')


class Slow(TuringTest):
    def test_halt(self):
        self._test_halt(HALT_SLOW)

    def test_spinout(self):
        self._test_spinout(SPINOUT_SLOW, fixed = False)
        self._test_spinout(SPINOUT_SLOW_FIXED, fixed = True)

    def test_recur(self):
        self._test_recur(RECUR_SLOW, quick = False)

    def test_macro(self):
        self._test_macro(MACRO_SLOW, quick = False)
