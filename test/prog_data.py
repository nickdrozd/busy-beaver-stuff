# pylint: disable = line-too-long, too-many-lines, consider-using-namedtuple-or-dataclass

from collections.abc import Mapping

## test turing #######################################################

BasicTermData = Mapping[str, tuple[int, int]]

HALT: BasicTermData = {
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
    "1RB 2RC 1LA  2LA 1RB 1R_  2RB 2RA 1LC": (95524079, 0),
    "1RB 2LA 1LC  0LA 2RB 1LB  1R_ 1RA 1RC": (374676383, 0),
}

SPINOUT: dict[str, tuple[int, int]] = {
    # 2/2
    "1RB 1LB  1LB 1LA": (3, 6),
    "1RB 0LB  1LB 1LA": (2, 6),
    "1RB 1LB  0LB 1LA": (2, 6),
    "1RB 0LB  0LB 1LA": (1, 6),

    # 3/2
    "1RB 0LB  1LA 0RC  1LC 1LA": (6, 55),  # BBB(3, 2)
    "1RB 0LB  1RC 0RC  1LC 1LA": (6, 54),
    "1RB 0LC  1LB 0RC  1LC 1LA": (5, 52),  # BB extension
    "1RB 0LC  0LC 0RC  1LC 1LA": (5, 51),
    "1RB 0LC  1LA 0RC  1RC 1RB": (5, 49),
    "1RB 0LC  0RC 0RC  1LC 1LA": (5, 48),
    "1RB 1LC  0RC ...  1LC 0LA": (5, 27),

    # 2/3
    "1RB 2LB 1LA  2LB 2RA 0RA": ( 8, 59),  # BBB(2, 3)
    "1RB 0LB 1RA  1LB 2LA 2RA": ( 3, 45),
    "1RB 2LB 1RA  2LB 2LA 0RA": (10, 43),
    "1RB 2RA 2LB  2LB 2LA 0LA": ( 5, 40),
    "1RB 1LB 1RA  2LB 2LA 0RA": ( 6, 23),
    "1RB 2RA 2LB  0LB 1LA 1RA": ( 4, 23),
    "1RB 2LB ...  1LB 2LA 1RB": ( 5, 17),

    # 4/2
    "1RB 1RC  1LC 1RD  1RA 1LD  0RD 0LB": (69, 2819),  # BBB sigma
    "1RB 0LC  1LD 0RC  1RA 0RB  0LD 1LA": (25, 1459),
    "1RB 1LC  1LC 0RD  1LA 0LB  1LD 0RA": (39, 1164),
    "1RB 1LB  1RC 0LD  0RD 0RA  1LD 0LA": (20, 1153),
    "1RB 0LB  0RC 0LC  0RD 1LC  1LD 0LA": (19,  673),
    "1RB 0LC  1LD 0RA  1RC 1RB  1LA 0LB": (31,  651),
    "1RB 1RC  1LD 0RA  0RC 1RD  1RA 0LB": (32,  581),
    "1RB 0LC  0RD 1LC  0LA 1LB  1LD 0RB": (22,  536),
    "1RB 0LB  1LB 1LC  1RD 0LB  1RA 0RD": (12,  444),

    # 2/4
    "1RB 2LB 3RA 2LA  3LB 3RA 0RB 1RB": (2747, 2501552),
    "1RB 2RB 1LA 0LB  2LB 3RB 0RB 1LA": ( 190,   32849),
    "1RB 2RB 3LA 2RA  1LB 1LA 1LB 3RB": (  62,   22464), # QH 22402
    "1RB 2RA 3LA 0LB  1LB 1LA 0RB 1RB": (  99,   16634),
    "1RB 2RB 1LA 1LA  2LB 2RA 3LB 1LA": (  62,    4067), # QH 4005
    "1RB 2RB 3LA 2RA  1LB 1LA 1LB 3RA": (  42,    3247),
    "1RB 2RB 3LA 2RA  1LB 1LA 2LB 3RA": (  42,    3057),
    "1RB 2RA 3LB 2LA  1LB 3LA 3RA 1RB": (  44,    3054),
    "1RB 2LB 3RA 0LA  1LB 2RB 2LA 1LA": (  31,    2872),
    "1RB 2RA 3LA 1LB  0LB 2LA 3RA 1RB": (  31,    2476),
    "1RB 2RA 2LA 3LB  0LB 1LA 3RB 0RA": (  30,    1854),
    "1RB 0RB 0LA 2LB  1LB 2LA 3RB 1RA": (  32,    1769),
    "1RB 0LA 0RB 2LB  3LB 3RA 0RA 1LA": (  36,    1525),
    "1RB 0LA 0RB 2LB  3LB 3RA 1RB 1LA": (  35,    1458),

    # 5/2
    "1RB 1RC  0LC 1RD  1LB 1LE  1RD 0RA  1LA 0LE": (19670, 193023636),
}

SPINOUT_BLANK = {
    # 2/2
    "1RB ...  1LB 0RB": ({'B'}, 4),

    # 3/2
    "1RB 1LB  1LA 1LC  1RC 0LC": ({'C'}, 34),
    "1RB 1LC  1LB 1LA  1RC 0LC": ({'C'}, 27),
    "1RB 1LB  1LA 1RC  1LC 0RC": ({'C'}, 26),
    "1RB 1LB  1LA 0LC  1RC 0LC": ({'C'}, 25),
    "1RB 0LB  1LA 1LC  1RC 0LC": ({'C'}, 23),
    "1RB 1LA  1LA 1RC  1LC 0RC": ({'C'}, 20),
    "1RB 1LA  1LA 1LC  1RC 0LC": ({'C'}, 20),
    "1RB 0LC  1LB 1LA  1RC 0LC": ({'C'}, 20),
    "1RB ...  1LC 0LC  1RC 0LB": ({'B', 'C'}, 20),

    # 2/3
    "1RB 2RA 2RB  2LB 1LA 0RB": ({'B'}, 29),
    "1RB 2RB 2RA  2LB 1LA 0RB": ({'B'}, 24),

    # 4/2
    "1RB 1LC  1RD 1RB  0RD 0RC  1LD 1LA": ({'C','D'}, 32779478),
    "1RB 0LC  1LD 0LA  1RC 1RD  1LA 0LD": ({'D','C'},    66349),
    "1RB 1RA  0RC 0RB  0RD 1RA  1LD 1LB": ({'B','C','D'}, 2568),
    "1RB 1RA  0RC 1LA  1LC 1LD  0RB 0RD": ({'B','C','D'}, 2512),
    "1RB 1LC  1LA 0RD  0RD 0RC  1LD 1LA": ({'C','D'},      712),
    "1RB 1LC  1LD 0RD  0RD 0RC  1LD 1LA": ({'C','D'},      710),
    "1RB 1LC  1RC 0RD  0RD 0RC  1LD 1LA": ({'C','D'},      705),
    "1RB 1LC  0RC 0RD  0RD 0RC  1LD 1LA": ({'C','D'},      703),
    "1RB 1LC  1LA 1RB  0RD 0RC  1LD 1LA": ({'C','D'},      535),
    "1RB 1LA  0LC 0LB  1RC 1RD  1LA 1RB": ({'B','C'},      496),
    "1RB 1LC  0RC 1RB  0RD 0RC  1LD 1LA": ({'C','D'},      456),
    "1RB 1RA  0RC 0RB  1LC 1LD  1RA 1LB": ({'B','C'},      427),
    "1RB ...  0RC 0LA  1LC 1LD  0RB 0RD": ({'B','C','D'},  171),
    "1RB 1LC  1RC 1LD  1LA 0LB  1RD 0LD": ({'D'},           77),
    "1RB 1LC  1LB 0RD  1RC 0LC  1LD 1LA": ({'C'},           66),

    # 2/4
    "1RB 2RB 3LA 2RA  2LB 1LA 0RB 3RA": ({'B'}, 1012664081),
    "1RB 2RA 1RA 2RB  2LB 3LA 0RB 2LA": ({'B'},     190524),
    "1RB 0RA 1RA 0RB  2LB 3LA 1LA 0RA": ({'B'},       4078),
    "1RB 2RA 3LA 2RB  2LB 1LA 0RB 0RA": ({'B'},       2501),
    "1RB 2RA 1RA 2LB  2LB 3LA 0RB 0RA": ({'B'},       1612),
    "1RB 0RA 1RA 0RB  2LB 3LA 0RB 0RA": ({'B'},       1538),
    "1RB 2RB 3RB 3RA  3LB 2LA 1RA 0RB": ({'B'},       1065),
    "1RB 2RB 1LA 0LA  2LB 3LA 0RB 1RA": ({'B'},        888),
    "1RB 2RA 1RA 2LB  2LB 3LA 0RB 2LA": ({'B'},        759),
    "1RB 0RA 1RA 0RB  2LB 3LA 1LA 2RA": ({'B'},        697),
    "1RB 0RA 1RA 2RB  2LB 3LA 0RB 0RA": ({'B'},        673),
    "1RB 2RA 1LA 0RB  2LB 3LA 0RB 2RA": ({'B'},        604),
    "1RB 2RA 1RA 2RB  2LB 3LA 0RB 2RA": ({'B'},        562),
    "1RB 2RA 1LA 2LB  2LB 3LA 0RB 2RA": ({'B'},        301),
    "1RB 2RB 2RA 1LA  2LB 3LA 0RB 1RA": ({'B'},        281),
    "1RB 2RB 1LA 1LB  2LB 3LA 0RB 2RA": ({'B'},        158),

    # 5/2
    "1RB 1RA  1RC ...  0RD 0RC  1LD 1LE  1RA 1LC": ({'C', 'D'}, 32738607),
    "1RB ...  1RC 1RB  0RD 0RC  1LD 1LE  1LA 1LC": ({'C', 'D'}, 32738620),
    "1RB 1LC  1RD 1RB  0RE 0RC  0RC ...  1LE 1LA": ({'C', 'E'}, 32748802),
    "1RB 1RC  1LD ...  0LE 0LC  0LC 1LD  1RE 1RA": ({'E', 'C'}, 32748816),
    "1RB ...  0LC 0LB  1RC 1RD  1LE 1RB  1LA 1LE": ({'B', 'C'}, 32759028),
    "1RB 1LA  0LC 0LB  1RC 1RD  1RE 1RB  1LA ...": ({'B', 'C'}, 32759042),
    "1RB 1RA  1LC ...  1RA 1LD  0RE 0RD  1LE 1LC": ({'E', 'D'}, 32769253),
    "1RB ...  1LC 1RB  1LA 1LD  0RE 0RD  1LE 1LC": ({'E', 'D'}, 32769267),
    "1RB ...  0RC 0RB  1LC 1LD  1RE 1LB  1RC 1RE": ({'B', 'C'}, 32779476),
    "1RB ...  1RC 1RB  1LC 1LD  1RB 1LE  0RC 0RE": ({'E', 'C'}, 32779478),
    "1RB 1RA  1RC ...  1LC 1LD  0RA 1LE  0RC 0RE": ({'E', 'C'}, 32779492),
    "1RB 1RC  1LD ...  0LE 0LC  1RE 1LD  1RE 1RA": ({'E', 'C'}, 32779493),
    "1RB 1RC  0LD ...  0LE 0LC  1LE 1LD  1RE 1RA": ({'E', 'C'}, 32779508),
    "1RB ...  1RC 1LD  0RE 1RC  0RE 0RD  1LE 1LB": ({'E', 'D'}, 32789703),
    "1RB ...  1LC 1RD  0LE 1LC  0LE 0LD  1RE 1RB": ({'E', 'D'}, 32789704),
    "1RB ...  0LC 1LB  1RC 1RD  1LB 1RE  0LC 0LE": ({'E', 'C'}, 32789706),
    "1RB ...  0LC 0LB  1RC 1RD  1LE 1RB  0LC 1LE": ({'B', 'C'}, 32789706),
    "1RB ...  1LC 1LB  0LD 0LC  1RD 1RE  0LB 1RC": ({'C', 'D'}, 32789716),
    "1RB ...  1RC 1RB  0RD 0RC  1LD 1LE  0RB 1LC": ({'C', 'D'}, 32789717),
    "1RB ...  0LC 1RD  1LD 1LC  0LE 0LD  1RE 1RB": ({'E', 'D'}, 32789719),
    "1RB ...  0LC 0LB  1RC 1RD  0LE 1RB  1LB 1LE": ({'B', 'C'}, 32789721),
    "1RB ...  0RC 1LD  0RD 1RC  0RE 0RD  1LE 1LB": ({'E', 'D'}, 32799940),
    "1RB ...  1LB 1LC  0RD 1LE  0RE 1RD  0RB 0RE": ({'E', 'B'}, 32799946),
    "1RB ...  1LB 1LC  1RD 1LE  0LE 1RD  0RB 0RE": ({'E', 'B'}, 32799961),
    "1RB 1LC  1RD 0LE  0RD 0RC  1LD 1LA  ... 1RA": ({'C', 'D'}, 32810048),
    "1RB ...  0RC 1LD  1LD 1RC  0RE 0RD  1LE 1LB": ({'E', 'D'}, 32810199),
    "1RB ...  0RC 0RB  1LC 1LD  0RE 1LB  1LB 1RE": ({'B', 'C'}, 32810203),
    "1RB ...  0RC 0RB  1LC 1LD  1RE 1LB  0LD 1RE": ({'B', 'C'}, 32810218),
    "1RB ...  0RC 1LD  1LB 1RC  0RE 0RD  1LE 1LB": ({'E', 'D'}, 32820458),
    "1RB ...  1LB 1LC  1RD 1LE  1LE 0RC  0RB 0RE": ({'E', 'B'}, 32830251),
    "1RB ...  1RC 1LB  1LD 1RD  0LE 0LD  1RE 0RB": ({'E', 'D'}, 32871346),
    "1RB ...  1LB 0LC  1LD 1RC  1RE 1LE  0RB 0RE": ({'E', 'B'}, 32871356),
    "1RB ...  1LC 1RB  1RD 1LD  0RE 0RD  1LE 0LB": ({'E', 'D'}, 32871358),
    "1RB 1LC  1RD 0LB  0RE 0RC  0RC ...  1LE 1LA": ({'E', 'C'}, 32891764),
    "1RB ...  1LB 1LC  0LD 1LE  1RD 0LE  0RB 0RE": ({'E', 'B'}, 32891776),

    # 5/2 constructed from 4/2 BLB
    "1RB 1LC  1RD 0LE  0RD 0RC  1LD 1LA  1RB 1RE": ({'C', 'D'}, 32810048),
    "1RB 1LC  0LD 1RB  0RE 0RC  1RE 1RD  1LE 1LA": ({'C', 'E'}, 32779508),

    # 6/2 inverted from 4/2 BLB
    "1RB ...  1RC ...  1LC 1LD  1RE 1LF  1RC 1RE  0RC 0RF": ({'C', 'F'}, 32779478),

}

SPINOUT_SLOW = {
    # 2/4
    "1RB 2RB 1LB 1LA  1LB 3RA 3LA 2RB": (3340, 2333909),

    # 5/2
   "1RB 0RC  1LC 0LD  1RE 0LD  0LC 1LB  0RE 1RA": (4843, 26181502),
}

SPINOUT_BLANK_SLOW = {
    # 2/4
    "1RB 2RA 1RA 2RB  2LB 3LA 0RB 0RA": ({'B'}, 1367361263049),

    # 5/2 blank
    "1RB 1RA  0RC 0RB  1LC 1LD  1RE 1LB  ... 0RA": (
        {'B', 'C'}, 348098678511),
    "1RB ...  0RC 1RB  1LC 1LD  1RE 0RD  0LE 0RB": (
        {'D', 'B', 'C'}, 455790469746),
    "1RB ...  0LC 0LB  0LD 1LC  1RD 0RE  1LB 1LA": (
        {'B', 'C', 'D'}, 455790469748),

    # 6/2 constructed from 4/2
    "1RB 1LC  1RD 1RB  0RE 1RE  1LD 1LA  0LF 1LF  0RD 0RC": (
        {'C', 'E', 'F', 'D'}, 65538552),
}

QUASIHALT = {
    # 3/2
    "1RB 1RC  1LC 0LB  1RA 1LA": (22,  2),  # center, >BB
    "1RB 1RC  1LC 1RA  1RA 1LA": ( 9,  2),  # center, >BB sigma
    "1RB ...  1LB 0LC  1RC 1RB": ( 5, 13),
    "1RB ...  1LB 1RC  0LC 0RB": ( 2, 14),
    "1RB ...  1LB 1LC  1RC 0RB": ( 2, 13),
    "1RB ...  1LC 0RB  1LB 1RC": ( 2, 10),

    # 2/3
    "1RB 1LA 2RA  2LA 2LB 2RB": (17, 2),
    "1RB 2LA 1RA  2LB 1LA 2RB": (16, 3),
    "1RB ... ...  2LB 1RB 1LB": ( 1, 5),

    # 4/2
    "1RB 1RC  1RD 0LC  1LD 0LD  1LB 0RA": (2332, (3, 1)),
    "1RB 0LC  1RC 1LD  1RD 0RB  0LB 1LA": (1460, (3, 2)),
    "1RB 0RC  0RD 1RA  0LD 0LA  1LC 1LA": ( 334, (2, 1)),
    "1RB 0RB  1LC 1RA  0LD 1LB  1RD 0LB": ( 119, 6),
    "1RB 1LC  1LD 0RA  1RC 0LD  0LC 1LA": ( 108, 8),
    "1RB 0LC  0RD 1RC  1LA 1RD  1LD 0RB": ( 105, 8),
    "1RB 1LA  1RC 1LD  1RD 0RC  1LB 0LA": ( 101, 8),

    # 5/2
    "1RB 1LC  1LC 1RA  1LB 0LD  1LA 0RE  1RD 1RE": (221032, (2, 1)),
    "1RB 1LC  0LC 0RD  1RD 1LE  1RE 1LA  1LA 0LB": ( 31317, (3, 1)),
    "1RB 1LC  1RD 1RA  1LB 0LA  1RE 0RC  1RC 0LE": (  3247, (3, 1)),

    # 2/4
    "1RB 2LB 2RA 3LA  1LA 3RA 3LB 0LB": (21485, (2,   1)),
    "1RB 2LA 1RA 1LA  2LB 3LA 2RB 2RA": (10456, (3, 104)),
    "1RB 2LA 1RA 1LA  3LA 1LB 2RB 2RA": ( 7193, (2,  88)),
    "1RB 2LA 1RA 1LA  3LA 1LB 2RB 2LA": ( 6443, (2,  82)),
    "1RB 0LA 1RA 0LB  2LB 3LA 2RB 0RA": ( 2859, (3,   1)),
}

RECUR_COMPACT = {
    # 2/2
    "1RB 0LB  1LA 0RB": (9, 3),
    "1RB 1LA  0LA 1RA": (7, 5),
    "1RB 1LB  1LA 0RB": (7, 3),
    "1RB 1LB  1LA 1RA": (5, 2),
    "1RB 0RB  1LB 1RA": (0, 9),
    "1RB 1LA  1LB 0RA": (0, 6),

    # 3/2
    "1RB 1LB  0RC 0LA  1LC 0LA": (101, 24),
    "1RB 1LA  1LC 1RC  1LA 0RB": ( 69, 16),
    "1RB 1LB  1RC 0LA  1LA 1RC": ( 65, 16),
    "1RB 0LC  1LC 1RB  1RA 1LA": ( 50, 16),
    "1RB 1LC  1LA 1RB  1RB 0LA": ( 50, 12),
    "1RB 0LC  1LC 1RB  1RB 1LA": ( 50, 12),
    "1RB 0LB  1LC 0RC  1RA 1LA": ( 38, 21),
    "1RB 1LA  0RC 0RA  1LC 0LA": ( 17, 36),
    "1RB ...  1LC 0RC  1RA 0LC": ( 16,  5),
    "1RB ...  1LC 1RA  1LA 0LC": ( 12,  8),
    "1RB ...  1LB 0LC  1LA 1RA": ( 12,  7),
    "1RB 1LB  0RC 0RB  1LC 0LA": (  4, 38),
    "1RB 0RB  1LC 0RC  0LA 1RA": (  2, 30),
    "1RB 0LA  0RC 1LA  1LC 0RB": (  0, 92),
    "1RB 0LA  1LB 0RC  1LC 1LA": (  0, 56),
    "1RB 0LA  0RC 0RC  1LC 1LA": (  0, 48),
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
    "1RB 2LB 2LA  2LA 0LB 0RA": ( 63, 32),
    "1RB 0RA 2LB  2LA 2RA 0LB": ( 59, 32),
    "1RB 1LB 1LB  1LA 2RB 0LA": ( 58,  8),
    "1RB 2LA 2LB  1LA 2RA 0LB": ( 57, 60),
    "1RB 1LA 2LB  2LA 2RA 0LB": ( 57, 30),
    "1RB 2LA 0RB  1LB 1RA 1RA": ( 55, 10),
    "1RB 0RB 0LB  2LA 2RA 1LB": ( 54, 40),
    "1RB 2LA 2RB  1LB 1LA 1RA": ( 39,  2),
    "1RB 2LA 0RB  2LA ... 1RA": ( 35,  8),
    "1RB 2LA 1RB  1LB 1LA 2RA": ( 24, 46),
    "1RB 1LA 2LB  1LA 2RA 0LB": ( 20, 48),
    "1RB 2RB 2LA  1LB 1RA 0LA": ( 14, 54),
    "1RB 2LA 1RB  1LB 1LA 0RA": (  7, 46),
    "1RB 0RA 1LB  2LA 2RB 0LA": (  6, 48),
    "1RB 0RA 2LB  2LA 0LA 1RA": (  5, 28),
    "1RB 1RA 0RB  2LB 1LA 1LB": (  4, 23),
    "1RB 2LA 0LB  1LA 2RA 2RB": (  3, 35),
    "1RB 2LB 2LA  1LA 2RB 0RA": (  1, 35),
    "1RB 2LB 0RA  1LA 2RB 2RA": (  0, 60),
    "1RB 2LB 0RA  1LA 1RB 2RA": (  0, 48),
    "1RB 2LA 1LB  0LA 0RB 1RA": (  0, 47),

    # 4/2
    "1RB 1LA  0RC 1RC  1LD 0RB  0LD 1LA": (586388, 104),
    "1RB 1RC  1LC 0LD  1RA 0LB  0RA 0RC": ( 14008,  24),
    "1RB 1LC  0RC 0RD  1LA 0LA  0LC 1RB": (  7002, 225),
    "1RB 0LA  0RC 0RD  1LC 1LA  0RB 1RD": (  3957, 265),
    "1RB 0LC  0RD 1RD  1LA 1LC  1RC 0RB": (  3316, 208),
    "1RB 0LC  1RD 1LD  0LA 1LB  1LC 0RD": (  6455,  23),
    "1RB 0LC  0RD 1RD  0LA 1LC  1LA 0RA": (  5252,   9),
    "1RB 0RC  1LD 0RA  0LD 0LB  1LA 1LB": (  4391,  24),
    "1RB 0LB  1LA 0LC  1LB 0RD  1RC 0RB": (  2374, 359),
    "1RB 0LA  1LC 0RA  0LD 1RD  1LA 0RB": (  2110,  36),
    "1RB 0LC  1RC 0RD  1LA 1LC  1RA 0RB": (  1978,   8),
    "1RB 1RC  1LC 0RB  1LD 0RA  1RA 0LB": (  1727, 622),
    "1RB 0LC  1RD 1RA  1LA 1LD  1LC 0RA": (  1709,  32),
    "1RB 0LA  0RC 1RD  1LD 0RB  1LA 1RD": (  1709,  13),
    "1RB 0RC  1LB 0LC  0RD 0LD  1RA 0LA": (  1680,   5),
    "1RB 0LC  1RD 0RD  1LA 0RC  1LB 1RC": (  1527, 522),
    "1RB 0LC  1RC 1RD  1LD 0RC  1LA 0RB": (  1301, 622),
    "1RB 1LC  1RD 0RB  0LC 1LA  1RC 0RA": (  1111, 131),
    "1RB 1RC  1LB 1LC  1RD 0LB  1RA 0RD": (  1033, 174),
    "1RB 0LC  1RD 0RB  1LC 1LA  1RC 1RA": (  1004, 174),
    "1RB 1LA  1RC 0RD  0LA 0RC  1RC 1LC": (   979, 144),
    "1RB 1RC  1LC 0LD  0RA 1LB  1RD 0LA": (   928, 128),
    "1RB 1LA  1RC 1LD  1RD 0RC  0LD 0LA": (   869, 404),
    "1RB 1LA  1LC 0RA  1LD 0LC  1RB 0LA": (   497, 816),
    "1RB 0RC  0LD 1RA  0LA 0RD  1LC 1LA": (   383, 200),
    "1RB 0LA  1LC 1LD  1RD 1LB  1RA 0RD": (    79, 481),
    "1RB 0LC  0RD 0RC  1LD 0RB  1LA 0LC": (    74, 945),
    "1RB 0LC  1RD 0RA  0LB 0LA  1LC 0RA": (    67, 945),
    "1RB 1LA  1RC 0RC  1LD 0RD  0LA 1LA": (    66, 284),
    "1RB 1RC  0RC 1RA  1LD 0RB  0LD 1LA": (    50, 597),
    "1RB 1RA  1LC 0RB  1RC 0LD  1LA 1LD": (    45, 228),
    "1RB 1LA  1LC 0RA  1LD 0LC  1RA 0LA": (     5, 385),
    "1RB 0RA  1LC 1RA  1LD 0LC  1LA 0RB": (     5, 244),
    "1RB 0LC  0RC 1LD  1RD 0LA  1LB 1LA": (     0, 294),
    "1RB 0LA  0RC 1LA  1RD 1RC  1LD 1LB": (     0, 714),
    "1RB 0LC  1LD 1LC  1RD 0LA  0RA 1LB": (     0, 294),
    "1RB 1LA  1LB 0RC  1LC 1LD  0RA 0LD": (     0, 238),
    "1RB 0LA  1LB 0RC  1RD 1RC  1LA 1LD": (     0, 228),

    # 2/4
    "1RB 2LA 3LA 1LA  2LB 3RA 0RA 2RB": (28284,  5),
    "1RB 2LA 0LB 1RA  1LB 3LA 3RB 3RB": (6697,  87),
    "1RB 2LB 0LA 1LA  2LA 3RA 1RB 0LB": ( 5632, 13),
    "1RB 1LB 2LA 3LA  1LA 2RB 3LB 0RA": ( 5281,  7),
    "1RB 0LA 2RB 0RB  3LB 2LA 1RA 1RA": (4996,  81),
    "1RB 2RA 0LB 1LA  3LA 2RB 1LA 1RA": (4702,  39),
    "1RB 0LB 1LB 1LB  2LA 3RB 1RA 0RA": (4632,  92),
    "1RB 2LA 3RB 2RB  3LA 3RA 0LB 1RA": (4325, 199),
    "1RB 2LB 1LA 0LB  3LA 3RA 1RB 0RA": (4300, 196),
    "1RB 2LB 1LA 0RB  3LA 2RA 3LB 1RB": (4111,  49),
    "1RB 0LB 1LB 2LA  2LA 0RA 3RA 2RB": (4050, 280),
    "1RB 2RB 3LB 0RA  1LA 3RB 2LA 2RA": ( 4000, 40),
    "1RB 2LB 1RA 3LA  2LA 0LA 3RB 1RA": (3665, 223),
    "1RB 2RB 0LB 1LA  3LA 3RA 1LA 1LB": (3439,  77),
    "1RB 2LB 3RA 1RA  3LA 0LB 1RA 0RA": (3294, 240),
    "1RB 2LB 3LA 0RB  2LA 1RA 1RB 2RA": (3231, 246),
    "1RB 2LB 3RA 0LB  1LA 3RA 3RB 2LA": ( 3010, 26),
    "1RB 2LA 3RA 2LB  2LA 2RA 3RB 0LA": ( 2991, 41),
    "1RB 2RA 1LB 2RB  2LA 2RB 3LA 0RA": ( 2983, 77),
    "1RB 2LB 0RA 2LB  2LA 3LA 0LB 3RA": ( 2973,290),
    "1RB 2LB 1RA 2LA  1LA 3RB 0RA 3LB": ( 2931,  8),
    "1RB 0RA 0LB 2RB  3LA 3RB 0LA 2RA": ( 2583,291),
    "1RB 0RB 2LB 1RA  3LA 1RA 3LB 2RB": ( 1089,  2),
}

RECUR_DIFFUSE = {
    # 4/2
    "1RB 0RA  1RC 0LB  1LD 0RD  1RA 1LB": (6836,  382),
    "1RB 0LC  0RC 1RC  1LA 0RD  1LC 0LA": (6825,  342),
    "1RB 0RA  1RC 0LD  0LB 1RA  0LA 1LD": (3115,  860),
    "1RB 0RC  1LB 1RC  1RA 0LD  1LA 1LC": ( 845,  842),
    "1RB 1RC  1LC 0RB  1RA 0LD  0LC 1LD": ( 600, 1374),

    # 2/4
    "1RB 2LA 1RB 0LB  1LA 3RA 3RB 1LB": (2380, 294),
    "1RB 2LB 0RA 2LB  2LA 3LA 0LB 0RA": (2190, 272),
}

RECUR_BLANK_IN_PERIOD = {
    # 2/2
    "1RB 0RA  1LB 1LA": (0, 8),
    "1RB 0RA  0LB 1LA": (0, 7),
    "1RB 1LA  0LA 0LB": (0, 7),
    "1RB 0LA  1LB 1RA": (0, 5),
    "1RB 0RA  1LA ...": (0, 4),
    "1RB 0RB  1LA 0LB": (None, 4),
    "1RB 1RB  1LA 0LB": (None, 4),

    # 3/2
    "1RB 1LB  0RC 1LA  1LA 0RA": (0, 21),
    "1RB ...  0RC 1LB  1LA 0RB": (None, 10),
    "1RB 0LB  1LA 1LC  0RC 0RB": (None,  7),

    # 2/3
    "1RB 2RB 0RA  2LA 1LA 1LB": (0, 27),
    "1RB 1LA 2RB  2LA 2RA 0LB": (None, 16),
    "1RB 2LA 0RB  1LA 0LB 1RA": (None,  4),

    # 2/4
    "1RB 2LB 3LA 0RA  1LA 3RB 3LB 2RA": (0, 224),
    "1RB 2LA 0RA 1LA  3LA 0LB 1RA 2LA": (None, 52),
}

RECUR_BLANK_BEFORE_PERIOD = {
    # 3/2
    "1RB 0RB  1LC 1RC  0LA 1LA",
    "1RB 0RB  1LC 0LC  1LA 1RA",
    "1RB 1LB  1LA 1RC  0RB 0LC",
    "1RB 1RC  1LC 0LB  1RA 1LA",
    "1RB 1LB  0LC 0RB  1RA 1LA",

    # 2/3
    "1RB 0RB ...  2LA ... 0LB",

    # 4/2
    "1RB 0RB  1LC 0LC  1RA 0LD  1LB 0LB",
    "1RB 1RA  1LC 0RD  1LB 1LD  1RA 0RB",

    # 5/2
    "1RB 1LC  0LC 0RD  1RD 1LE  1RE 1LA  1LA 0LB",
    "1RB 1LC  1RD 1RA  1LB 0LA  1RE 0RC  1RC 0LE",
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

RECUR_TOO_SLOW = {
    "1RB 0LC  1RD 1LC  0LA 1LB  1LC 0RD": (309086174, 7129704),
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

SPAGHETTI = {
    # Halt
    "1RB 2RA 2RC  1LC 1R_ 1LA  1RA 2LB 1LC",  # 310341163

    "1RB 1LA 3LA 3RC  2LC 2LB 1RB 1RA  2LA 3LC 1R_ 1LB",
    "1RB 3LA 3RC 1RA  2RC 1LA 1R_ 2RB  1LC 1RB 1LB 2RA",

    # Quasihalt
    "1RB 2LC 2RA  1LA 2LB 1RC  1RA 2LC 1RB",  # marks
    "1RB 0RC 1RA  2LB 2LC 0RA  0RB 2LA 0RC",
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

    "1RB 1LA 1RD  2LC 0RA 1LB  2LA 0LB 0RD  2RC 1R_ 0LC": 3,

    "1RB 1RA 1LB 1RC  2LA 0LB 3LC 1R_  1LB 0RC 2RA 2RC": 3,

    "1RB 0LD  1RC 0RF  1LC 1LA  0LE 1R_  1LF 0RB  0RC 0RE": 4,  # Pavel, BB6
    "1RB 1RC  1LC 0RF  1RA 0LD  0LC 0LE  1LD 0RA  1RE 1R_": 4,  # Shawn
    "1RB 0LD  1RC 0RF  1LC 1LA  0LE 1R_  1LA 0RB  0RC 0RE": 3,  # Pavel
    "1RB 0LC  1LA 1RD  1RA 0LE  1RA 0RB  1LF 1LC  1RD 1R_": 3,
    "1RB 0LC  1LA 1RD  0LB 0LE  1RA 0RB  1LF 1LC  1RD 1R_": 3,
    "1RB 0RC  0LA 0RD  1RD 1R_  1LE 0LD  1RF 1LB  1RA 1RE": 3,

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

    # Prover false positive
    "1RB 0RD  1LC 0RA  1LA 1LB  1R_ 0RC": 3,
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

CANT_HALT_FALSE_NEGATIVES: set[str] = {
    "1RB 0RA  1LA ...",

    "1RB ...  1LC 1RA  1LA 0LC",
    "1RB ...  1LC 0RC  1RA 0LC",

    "1RB 2LB ...  1LB 2LA 1RB",
    "1RB 0LA ...  1LB 2LA 0RB",
    "1RB 2LA 0RB  2LA ... 1RA",

    "1RB 1RA  1RC ...  0RD 0RC  1LD 1LE  1RA 1LC",
    "1RB ...  1RC 1RB  0RD 0RC  1LD 1LE  1LA 1LC",
    "1RB 1LC  1RD 1RB  0RE 0RC  0RC ...  1LE 1LA",
    "1RB 1RC  1LD ...  0LE 0LC  0LC 1LD  1RE 1RA",
    "1RB ...  0LC 0LB  1RC 1RD  1LE 1RB  1LA 1LE",
    "1RB 1LA  0LC 0LB  1RC 1RD  1RE 1RB  1LA ...",
    "1RB 1RA  1LC ...  1RA 1LD  0RE 0RD  1LE 1LC",
    "1RB ...  1LC 1RB  1LA 1LD  0RE 0RD  1LE 1LC",
    "1RB 1RA  1RC ...  1LC 1LD  0RA 1LE  0RC 0RE",
    "1RB 1RC  1LD ...  0LE 0LC  1RE 1LD  1RE 1RA",
    "1RB 1RC  0LD ...  0LE 0LC  1LE 1LD  1RE 1RA",
}

CANT_BLANK_FALSE_NEGATIVES: set[str] = {
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
    "1RB 0LE  1RC 1LB  0RD 0RB  1LD 0LE  0RC 1LA",

    "1RB 1RA 2LB 3LA  2LA 0LB 1LC 1LB  3RB 3RC 1R_ 1LC",
    "1RB 1RA 1LB 1RC  2LA 0LB 3LC 1R_  1LB 0RC 2RA 2RC",

    "1RB 0LB 1RD  2RC 2LA 0LA  1LB 0LA 0LA  1RA 0RA 1R_",
}

CANT_SPIN_OUT_FALSE_NEGATIVES: set[str] = {
    "1RB 0RB 0LB  1LB 2RA 1LA",
    "1RB 2LA 0RB  0LB 1LA 0RA",

    # slow
    "1RB ... ...  2LB 1RB 1LB",
    "1RB 1RA 0RB  2LB 1LA 1LB",

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

DO_HALT: set[str] = {
    # 3/4
    "1RB 0LB 1R_ 3LA  0LC 3RB 3RC 1LB  2RB 2LA 3RA 1LC",  # 10^^2048

    # 2/6
    "1RB 3RB 5RA 1LB 5LA 2LB  2LA 2RA 4RB 1R_ 3LB 2LA",  # 10^^^3
    "1RB 3LA 4LB 0RB 1RA 3LA  2LA 2RA 4LA 1RA 5RB 1R_",  # 10^^90
    "1RB 2LA 1RA 4LA 5RA 0LB  1LA 3RA 2RB 1R_ 3RB 4LA",  # 10^^70

    # 6/2
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

    # 2/5
    "1RB 2LB 4LB 3LA 1R_  1LA 3RA 3LB 0LB 0RA",  # 10^19017
    "1RB 2LA 1RA 2LB 2LA  0LA 2RB 3RB 4RA 1R_",  # 10^704
    "1RB 2LA 4RA 2LB 2LA  0LA 2RB 3RB 4RA 1R_",  # 10^211
    "1RB 2LA 4RA 2LB 2LA  0LA 2RB 3RB 1RA 1R_",  # 10^211
    "1RB 2LA 1RA 2LB 3RA  0LA 2RB 4RB 1R_ 2RA",  # 10^176
    "1RB 2LA 1RA 2LB 2RA  0LA 3RB 3RB 4RA 1R_",  # 10^113
    "1RB 2LA 1RA 2LB 3RA  0LA 2RB 4RB 1R_ 2LA",  # 10^101
    "1RB 2LA 4RA 1LB 2LA  0LA 2RB 3RB 2RA 1R_",  # 10^61
    "1RB 2LA 1RA 2RB 2LB  0LA 3RA 4RB 1R_ 1RA",  # 10^60
    "1RB 2LA 1RA 2LB 2RA  0LA 2RB 3RB 4RA 1R_",  # 10^52
    "1RB 4LA 1LA 1R_ 2RB  2LB 3LA 1LB 2RA 0RB",  #      7,021,292,621
    "1RB 3LA 1LA 4LA 1RA  2LB 2RA 1R_ 0RA 0RB",  # 26,375,397,569,930
}

DONT_BLANK: set[str] = {
    "1RB 2LA 1LA  2LA 2RB 0RA",  # wolfram
}

DO_BLANK: set[str] = {
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

DO_SPIN_OUT: set[str] = {
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

DONT_SPIN_OUT: set[str] = {
    "1RB 2LA 1LA  2LA 2RB 0RA",  # wolfram

    "1RB 0LB  1RC 0RC  0RD 1LA  1LE 1RD  0LC 0RE",  # 10^26
    "1RB 1RD  1LC 1LB  1LD 1RA  0RE 0RD  1LB 1RE",  # 10^28, xmas
    "1RB 1RA  1LC 0RB  1LE 0LD  1RA 1RE  1LB 0RA",  # 10^31
    "1RB 1LE  0RC 1LD  1RD 0RD  1RE 1RC  0LA 1LB",  # 10^46
}

BLOCK_MACRO_STEPS = {
    # 2/4
    "1RB 2LA 1RA 1RA  1LB 1LA 3RB 1R_": 3932964,

    # 4/2
    "1RB 1LC  1RD 1RB  0RD 0RC  1LD 1LA": 32779478,

    # 5/2
    "1RB 1LC  1RC 1RB  1RD 0LE  1LA 1LD  1R_ 0LA": 47176870,
}

MacroCycles = Mapping[
    str | tuple[str, int | None],
    tuple[int | None, ...],
]

MACRO_CYCLES_FAST: MacroCycles = {
    # 2/4 BB
    "1RB 2LA 1RA 1RA  1LB 1LA 3RB 1R_": (
        12233,  # base
        12200,  # 2-cell
        12845,  # 3-cell
        14224,  # back
        12138,  # 2-cell back
        12756,  # 3-cell back
        14196,  # back back
        14190,  # back 2-cell
        12810,  # back 3-cell
    ),

    # 4/2 BLB, SB, BBB
    "1RB 1LC  1RD 1RB  0RD 0RC  1LD 1LA" : (
        25583,  # base
        30663,  # 2-cell
        25377,  # 3-cell
        35750,  # back
        30625,  # 2-cell back
        25329,  # 3-cell back
        35734,  # back back
        25528,  # back 2-cell
        25359,  # back 3-cell
    ),

    # 4/2 SB sig, BBB sig
    "1RB 1RC  1LC 1RD  1RA 1LD  0RD 0LB": (
        2819,  # base
        1161,  # 2-cell
        944,   # 3-cell
        2076,  # back
        156,   # 2-cell back
        443,   # 3-cell back
        1382,  # back back
        253,   # back 2-cell
        695,   # back 3-cell
    ),

    # 4/2 former BLB, SB, BBB
    "1RB 0LC  1LD 0LA  1RC 1RD  1LA 0LD": (
        2221,  # base
        1687,  # 2-cell
        1448,  # 3-cell
        1208,  # back
        1005,  # 2-cell back
        882,   # 3-cell back
        1201,  # back back
        879,   # back 2-cell
        896,   # back 3-cell
    ),

    # 4/2 boyd
    ("1RB 0RC  1LB 1LD  0RA 0LD  1LA 1RC", 20_000): (
        20000,  # base
        20194,  # 2-cell
        20000,  # 3-cell
        22917,  # back
        21104,  # 2-cell back
        24127,  # 3-cell back
        21336,  # back back
        20285,  # back 2-cell
        21402,  # back 3-cell
    ),

    # 4/2 LR start, period
    ("1RB 0LC  1RD 1LC  0LA 1LB  1LC 0RD", 20_000): (
        23855,  # base
        21932,  # 2-cell
        20540,  # 3-cell
        22981,  # back
        22589,  # 2-cell back
        20403,  # 3-cell back
        22388,  # back back
        22232,  # back 2-cell
        20593,  # back 3-cell
    ),

    # 5/2 BB
    "1RB 1LC  1RC 1RB  1RD 0LE  1LA 1LD  1R_ 0LA": (
        67345,  # base
        48182,  # 2-cell
        26527,  # 3-cell
        73423,  # back
        48140,  # 2-cell back
        26491,  # 3-cell back
        73400,  # back back
        48158,  # back 2-cell
        32599,  # back 3-cell
    ),

    # 5/2 Uwe
    "1RB 0LC  1RC 1RD  1LA 0RB  0RE 1R_  1LC 1RA": (
        134466,  # base
        2415,    # 2-cell
        44822,   # 3-cell
        133806,  # back
        2373,    # 2-cell back
        44172,   # 3-cell back
        133151,  # back back
        2382,    # back 2-cell
        44607,   # back 3-cell
    ),

    # 5/2 total spaghetti
    ("1RB 1LC  0LC 0RD  1RD 1LE  1RE 1LA  1LA 0LB", None): (
        None,  # base
        None,  # 2-cell
        None,  # 3-cell
        695,   # back
        551,   # 2-cell back
        525,   # 3-cell back
        678,   # back back
        558,   # back 2-cell
        550,   # back 3-cell
    ),

    # 3/3
    "1RB 2RA 2RC  1LC 1R_ 1LA  1RA 2LB 1LC": (
        276588,  # base
        195229,  # 2-cell
        142455,  # 3-cell
        180399,  # back
        99035,   # 2-cell back
        64126,   # 3-cell back
        144319,  # back back
        117455,  # back 2-cell
        81409,   # back 3-cell
    ),
}

MACRO_CYCLES_SLOW: MacroCycles = {
    # 2/4
    "1RB 2RB 1LB 1LA  1LB 3RA 3LA 2RB": (
        2329499,  # base
        1162389,  # 2-cell
        20520,    # 3-cell
        2322228,  # back
        1153855,  # 2-cell back
        11399,    # 3-cell back
        2312445,  # back back
        1157869,  # back 2-cell
        17966,    # back 3-cell
    ),

    # 3/3
    "1RB 1R_ 2RB  1LC 0LB 1RA  1RA 2LC 1RC": (
        503496,  # base
        323669,  # 2-cell
        388079,  # 3-cell
        611312,  # back
        323611,  # 2-cell back
        387985,  # 3-cell back
        611277,  # back back
        323643,  # back 2-cell
        402084,  # back 3-cell
    ),

    # 3/3 surprise-in-a-box
    "1RB 2LB 1LC  1LA 2RB 1RB  1R_ 2LA 0LC": (
        1691774,  # base
        1061760,  # 2-cell
        824186,   # 3-cell
        1616856,  # back
        622359,   # 2-cell back
        360281,   # 3-cell back
        1201808,  # back back
        881136,   # back 2-cell
        768963,   # back 3-cell
    ),

    # 6/2 pessimized from 4/2
    "1RB 1LC  1RD 1RB  0RE 1RE  1LD 1LA  0LF 1LF  0RD 0RC": (
        49153991,  # base
        24588500,  # 2-cell
        16391915,  # 3-cell
        35750,     # back
        30625,     # 2-cell back
        25329,     # 3-cell back
        35734,     # back back
        25528,     # back 2-cell
        25359,     # back 3-cell
    ),
}

BLANKERS = (
    {prog: None for prog in DO_BLANK}
    | SPINOUT_BLANK
    | SPINOUT_BLANK_SLOW
    | RECUR_BLANK_IN_PERIOD
    | {prog: None for prog in RECUR_BLANK_BEFORE_PERIOD}
)

DIFFUSE = {
    # 5/2
    "1RB 1LC  1RC 1RB  1RD 0LE  1LA 1LD  1R_ 0LA": 3,  # BB(5)
    "1RB 0LC  1RC 1RD  1LA 0RB  0RE 1R_  1LC 1RA": 2,  # uwe
    "1RB 1RC  0LC 1RD  1LB 1LE  1RD 0RA  1LA 0LE": 3,  # high mark SO

    # 3/3
    "1RB 1R_ 2RB  1LC 0LB 1RA  1RA 2LC 1RC": 2,
    "1RB 2LA 1RA  1RC 2RB 0RC  1LA 1R_ 1LA": 2,

    # QH
    "1RB ...  1LB 1RC  0LC 0RB": 2,

    "1RB 0RC  0RD 1RA  0LD 0LA  1LC 1LA": 2,
    "1RB 0RB  1LC 1RA  0LD 1LB  1RD 0LB": 2,
    "1RB 1LC  1LD 0RA  1RC 0LD  0LC 1LA": 2,
    "1RB 0LC  0RD 1RC  1LA 1RD  1LD 0RB": 2,
    "1RB 1LA  1RC 1LD  1RD 0RC  1LB 0LA": 2,

    "1RB 1LC  1LC 1RA  1LB 0LD  1LA 0RE  1RD 1RE": 3,

    # 4/2
    "1RB 0RC  1LB 1LD  0RA 0LD  1LA 1RC":    118,  # boyd
}

ProverEst = dict[
    str,
    int | tuple[float, int] | str,
]

PROVER_HALT: ProverEst = {
    # 2/5
    "1RB 2LB 4LB 3LA 1R_  1LA 3RA 3LB 0LB 0RA": (7.3, 19016),
    "1RB 2LA 1RA 2LB 2LA  0LA 2RB 3RB 4RA 1R_": (1.7, 352),
    "1RB 2LA 4RA 2LB 2LA  0LA 2RB 3RB 4RA 1R_": (5.2, 105),
    "1RB 2LA 4RA 2LB 2LA  0LA 2RB 3RB 1RA 1R_": (5.2, 105),
    "1RB 2LA 3RA 2LA 2LB  0LA 2RB 4RB 1R_ 1RA": (5.2, 105),
    "1RB 2LA 1RA 2LB 3RA  0LA 2RB 4RB 1R_ 2RA": (2.4,  88),
    "1RB 2LA 1RA 2LB 2RA  0LA 3RB 3RB 4RA 1R_": (6.2,  56),
    "1RB 2LA 1RA 2LB 3RA  0LA 2RB 4RB 1R_ 2LA": (1.0,  51),
    "1RB 2LA 4RA 1LB 2LA  0LA 2RB 3RB 2RA 1R_": (9.3,  30),
    "1RB 2LA 1RA 2RB 2LB  0LA 3RA 4RB 1R_ 1RA": (4.0,  30),
    "1RB 2LA 1RA 2LB 2RA  0LA 2RB 3RB 4RA 1R_": (3.2,  26),

    # 2/6
    "1RB 2LA 1RA 4LA 5RA 0LB  1LA 3RA 2RB 1R_ 3RB 4LA": "(14 + (2 ** (-1",  # 10 ^^ 70
    "1RB 2LA 1R_ 5LB 5LA 4LB  1LA 4RB 3RB 5LB 1LB 4RA": (1.9, 4933),
    "1RB 1LB 3RA 4LA 2LA 4LB  2LA 2RB 3LB 1LA 5RA 1R_": (6.9, 4931),
    "1RB 2LB 4RB 1LA 1RB 1R_  1LA 3RA 5RA 4LB 0RA 4LA": (8.6,  821),
    "1RB 0RB 3LA 5LA 1R_ 4LB  1LA 2RB 3LA 4LB 3RB 3RA": (1.9,   27),

    # 4/3
    "1RB 1R_ 2RC  2LC 2RD 0LC  1RA 2RB 0LB  1LB 0LD 2RC": (1.3, 7036),
    "1RB 0LB 1RD  2RC 2LA 0LA  1LB 0LA 0LA  1RA 0RA 1R_": (4.2, 6034),
    "1RB 1LD 1R_  1RC 2LB 2LD  1LC 2RA 0RD  1RC 1LA 0LA": (8.9, 4931),
    "1RB 2LD 1R_  2LC 2RC 2RB  1LD 0RC 1RC  2LA 2LD 0LB": (2.5, 4561),
    "1RB 1LA 1RD  2LC 0RA 1LB  2LA 0LB 0RD  2RC 1R_ 0LC": (4.0, 3860),

    # 3/4
    "1RB 1RA 2LB 3LA  2LA 0LB 1LC 1LB  3RB 3RC 1R_ 1LC": (3.7, 6518),
    "1RB 1RA 1LB 1RC  2LA 0LB 3LC 1R_  1LB 0RC 2RA 2RC": (2.2, 2372),
    "1RB 1LA 3LA 3RC  2LC 2LB 1RB 1RA  2LA 3LC 1R_ 1LB": (1.7, 1301),
    "1RB 3LA 3RC 1RA  2RC 1LA 1R_ 2RB  1LC 1RB 1LB 2RA": (2.1,  628),
    "1RB 0RB 3LC 1RC  0RC 1R_ 2RC 3RC  1LB 2LA 3LA 2RB": (4.6,  434),
    "1RB 1LA 1LB 1RA  0LA 2RB 2LC 1R_  3RB 2LB 1RC 0RC": (2.4,   26),

    # 6/2
    "1RB 1R_  0LC 0LD  1LD 1LC  1RE 1LB  1RF 1RD  0LD 0RA": (1.7, 646_456_993),
    "1RB 1R_  1RC 1RA  1RD 0RB  1LE 0RC  0LF 0LD  0LB 1LA": (2.0, 98641),
    "1RB 1RC  1LC 0RF  1RA 0LD  0LC 0LE  1LD 0RA  1RE 1R_": (6.0, 39456),
    "1RB 1LE  1RC 1RF  1LD 0RB  1RE 0LC  1LA 0RD  1R_ 1RC": (3.5, 18267),
    "1RB 0LD  1RC 0RF  1LC 1LA  0LE 1R_  1LA 0RB  0RC 0RE": (3.1, 10566),
    "1RB 0LE  1LC 0RA  1LD 0RC  1LE 0LF  1LA 1LC  1LE 1R_": (4.6,  1439),
    "1RB 0RF  0LB 1LC  1LD 0RC  1LE 1R_  1LF 0LD  1RA 0LE": (2.5,   881),
    "1RB 0LF  0RC 0RD  1LD 1RE  0LE 0LD  0RA 1RC  1LA 1R_": (1.2,   865),
    "1RB 0LB  0RC 1LB  1RD 0LA  1LE 1LF  1LA 0LD  1R_ 1LE": (6.4,   462),
    "1RB 0LC  1LA 1RC  1RA 0LD  1LE 1LC  1RF 1R_  1RA 1RE": (1.4,    60),
    "1RB 0LB  1LC 0RE  1RE 0LD  1LA 1LA  0RA 0RF  1RE 1R_": (6.9,    49),
    "1RB 0LC  1LA 1LD  1RD 0RC  0LB 0RE  1RC 1LF  1LE 1R_": (1.1,    49),
    "1RB 0LC  1LA 1RD  1RA 0LE  1RA 0RB  1LF 1LC  1RD 1R_": (6.7,    47),
    "1RB 0LC  1LA 1RD  0LB 0LE  1RA 0RB  1LF 1LC  1RD 1R_": (6.7,    47),
    "1RB 0RC  0LA 0RD  1RD 1R_  1LE 0LD  1RF 1LB  1RA 1RE": (2.5,    21),
}

PROVER_SPINOUT: ProverEst = {
    # 2/4
    "1RB 2RA 1LA 2LB  2LB 3RB 0RB 1RA": 530843045,
    "1RB 2LA 1RA 1LB  0LB 2RB 3RB 1LA": 414095476548,

    # 3/3
    "1RB 0LB 1LA  2LC 2LB 2LB  2RC 2RA 0LC": 0,
    "1RB 2RA 0RB  2LA 0RA 1RC  1LC 0RC 0LA": 0,
    "1RB 2RA 0RB  2LA 1RA 1RC  1LC 0RC 0LA": 0,
    "1RB 0LB 1LA  2LC 0LB 2LB  2RC 2RA 0LC": 0,
    "1RB 0LB 1LA  2LC 0LA 2LB  2RC 2RA 0LC": 0,
    "1RB 2RA 1LC  2LB 0RB 2LA  1LA 0LB 1LA": 476915187810,
    "1RB 2RA 1LC  2LB 0RB 2LA  2RB 0LB 1LA": 476915187810,
    "1RB 0LB 1RA  2LA 0LC 0LB  2RC 0LA 1LC": 0,
    "1RB 2LB 2RA  2LA 0RA 2LC  2LC 1LA 1RC": 3193555698,
    "1RB 1LC 0LC  0RC 2RB 2RA  2LC 1LA 1RC": 2502976232,
    "1RB 2LA 1LB  2LB 1LA 1RC  1RC 0LC 2RA": 0,
    "1RB 1RC 0RC  1RC 0LA 1LB  2LC 2RA 1LB": 39598896,
    "1RB 1LA 1LC  0LB 2RB 1RC  1LA 1RC 1LB": 16621756,
    "1RB 0RC 1RA  2LB 2LC 0RA  0RB 2LA 0RC": 0,
    "1RB 2RA 0RB  2LB 1LC 0LA  1LA 2RA 2LA": 7086608,
    "1RB 0LB 0RB  2RC 0RA 1RB  2LC 1LB 0RC": 0,

    # 5/2
    "1RB 1LE  1RC 0RA  0LD 0LC  1RD 1RE  1LB 0RB": 1195275720475,
    "1RB 0LE  1RC 1LB  0RD 0RB  1LD 0LE  0RC 1LA": 44776124,
    "1RB 0RE  1RC 0RB  1LD 0RA  0LA 0LC  0LE 1LA": 23914766,
    "1RB 0LE  0RC 1RB  0RD 1RA  1LD 1LA  1LC 0RB": 0,
    "1RB 0LE  0RC 0LC  0RD 1RA  1LD 1LA  1LC 0RB": 0,
    "1RB 0RB  1RC 1RB  1LC 1LD  1RA 1LE  0RC 0RE": 0,
    "1RB 0LC  1RC 0LA  1LD 0RB  0RE 0RD  1LE 0LA": 0,
    "1RB 1LE  0RC 0RD  1LC 1LA  0RB 1RD  0LD 0RE": 2,
    "1RB 0RB  0RC 0RB  0RD 0RE  1LD 0LE  1LA 1RE": 0,
    "1RB 1LE  0RC 0RB  0RD 0RE  1LD 0LA  1LB 1RE": 0,
    "1RB 1LA  1LC 1RC  0LD 0LC  1RE 0LA  1RE 0RA": 2,
    "1RB 0RE  0RC 0RB  1LC 0LD  1RD 1LA  1LB 1RE": 0,
    "1RB 0RE  0RC 0RB  0RD 0RE  1LD 0LA  0RB 1RE": 0,
    "1RB 0RE  0LC 0LB  1RC 1RD  1LA 0RA  1RA 1LD": (7.0, 41),
    "1RB 1RA  1LB 0LC  1RC 1LD  0RE 0RA  0RB 0RD": 0,
    "1RB 1LA  0LC 0LB  0LD 1LC  1RE 0LA  1RE 0RA": 2,
    "1RB 0LC  0LD 0LB  1RA 1LA  1RE 1LD  1RE 1RA": 2,
    "1RB 1LC  1RC 0RD  0LB 0RC  0RE 1RD  1LE 1LA": 0,
    "1RB 1LC  0RD 0RD  0LB 0RC  0RE 1RD  1LE 1LA": 0,
    "1RB 1LC  1RD 0RA  0LC 1LE  1LA 0RE  0LA 1RB": (3.2, 544),
    "1RB 1LC  0LD 0LB  0RE 0LA  0LE 1LD  1RE 1RA": 2,
    "1RB 1LC  0LD 0LB  1RE 0LA  0LC 1LD  1RE 1RA": 2,
    "1RB 1LC  0LD 0LB  0LE 0LA  0LE 1LD  1RE 1RA": 2,
    "1RB 1LC  0LD 0LB  0RD 0LA  0LE 1LD  1RE 1RA": 2,
}

PROVER_QUASIHALT = {
    "1RB 2LC 2RA  1LA 2LB 1RC  1RA 2LC 1RB",
    "1RB 2LA 1RA  1RC 2RB 2RC  1LA 1LB 1LC",
    "1RB 2RC 1LA  2LA 1RB 2LB  2RB 2RA 1LC",
    "1RB 2RC 1LA  2LA 1RB 0RB  2RB 2RA 1LC",
}

RULE_LIMIT = {
    # 3/4
    "1RB 0LB 1R_ 3LA  0LC 3RB 3RC 1LB  2RB 2LA 3RA 1LC",  # 10^^2048

    # 2/6
    "1RB 3RB 5RA 1LB 5LA 2LB  2LA 2RA 4RB 1R_ 3LB 2LA",  # 10^^^3
    "1RB 3LA 4LB 0RB 1RA 3LA  2LA 2RA 4LA 1RA 5RB 1R_",  # 10^^90
    "1RB 2RB 3RB 4RB 5LA 4RA  0LA 1RB 5RA ... ... 1LB",  # ???

    # 6/2
    "1RB 0LD  1RC 0RF  1LC 1LA  0LE 1R_  1LF 0RB  0RC 0RE",  # 10^^15
    "1RB 0LA  1LC 1LF  0LD 0LC  0LE 0LB  1RE 0RA  1R_ 1LD",  # 10^^5
    "1RB 1RE  1LC 1LF  1RD 0LB  1LE 0RC  1RA 0LD  1R_ 1LC",
}

PROVER_FAILURES = {
    "1RB 1RC 0RC  1RC 0LA 1LB  2LC 2RA 1LB",

    "1RB 1LB 3RA 4LA 2LA 4LB  2LA 2RB 3LB 1LA 5RA 1R_",
    "1RB 3RB 5RA 1LB 5LA 2LB  2LA 2RA 4RB 1R_ 3LB 2LA",

    "1RB 1LD 1R_  1RC 2LB 2LD  1LC 2RA 0RD  1RC 1LA 0LA",
}

ALGEBRA_STRINGS: dict[str, tuple[int, str]] = {
    "1RB 1R_  0LC 0LD  1LD 1LC  1RE 1LB  1RF 1RD  0LD 0RA": (
        522,
        "(1 + (2 ** 2147483647))",
    ),
    "1RB 0LA  1LC 1LF  0LD 0LC  0LE 0LB  1RE 0RA  1R_ 1LD": (
        3344,
        "(208787669 + ((52195782 * (8 ** (4390 + ((23612377 * (8 ** 23050)) + ((-831488 + (203 * (8 ** 23050))) // 189))))) + ((-286720 + (70 * (8 ** (4390 + ((23612377 * (8 ** 23050)) + ((-831488 + (203 * (8 ** 23050))) // 189)))))) // 63)))",
    ),
    "1RB 0LB 1R_ 3LA  0LC 3RB 3RC 1LB  2RB 2LA 3RA 1LC": (
        655,
        "(4088 + (2 ** (1 + (2 ** (2 + (2 ** (2 + (2 ** 524290))))))))",
    ),
    "1RB 1R_  1RC 1RA  1RD 0RB  1LE 0RC  0LF 0LD  0LB 1LA": (
        291,
        "(-5 + (5 * (2 ** 327677)))",
    ),
    "1RB 1RC  1LC 0RF  1RA 0LD  0LC 0LE  1LD 0RA  1RE 1R_": (
        1698,
        "(3 * (2 ** 131071))",
    ),
    "1RB 1LE  1RC 1RF  1LD 0RB  1RE 0LC  1LA 0RD  1R_ 1RC": (
        5723,
        "(2126281 + (((~10^15) * (8 ** 20210)) + ((-8707112960 + (33215 * (8 ** 20210))) // 4095)))",
    ),
    "1RB 0LD  1RC 0RF  1LC 1LA  0LE 1R_  1LA 0RB  0RC 0RE": (
        413,
        "(107 + ((3 ** 22145) + ((-15309 + (7 * (3 ** 22149))) // 162)))",
    ),
    "1RB 0LB  0RC 1LB  1RD 0LA  1LE 1LF  1LA 0LD  1R_ 1LE": (
        3067,
        "(~10^463)",
    ),
    "1RB 2LA 1R_ 5LB 5LA 4LB  1LA 4RB 3RB 5LB 1LB 4RA": (
        731,
        "(2 ** 16388)",
    ),
    "1RB 2LB 4RB 1LA 1RB 1R_  1LA 3RA 5RA 4LB 0RA 4LA": (
        937,
        "(111 + ((681 * (2 ** 2720)) + ((-320 + (5 * (2 ** 2720))) // 3)))",
    ),
    "1RB 1RA 2LB 3LA  2LA 0LB 1LC 1LB  3RB 3RC 1R_ 1LC": (
        347,
        "(26 + ((1003 * (3 ** 13656)) + ((-57 + (19 * (3 ** 13656))) // 2)))",
    ),
    "1RB 1LA 3LA 3RC  2LC 2LB 1RB 1RA  2LA 3LC 1R_ 1LB": (
        227,
        "(16 + ((1617 * (2 ** 4312)) + ((-52 + (13 * (2 ** 4312))) // 3)))",
    ),
    "1RB 3LA 3RC 1RA  2RC 1LA 1R_ 2RB  1LC 1RB 1LB 2RA": (
        681,
        "(~10^628)",
    ),
    "1RB 2RB 3RB 4RB 5LA 4RA  0LA 1RB 5RA ... ... 1LB": (
        957,
        "(2 ** (-1 + (2 ** (2 ** (2 ** 65536)))))",
    ),
    "1RB 1RA  1LC 1RE  0RE 1LD  1RE 1LC  1LA 0LE": (
        434,
        "(-3 + (2 ** (2 + (2 ** (1 + (2 ** (1 + (2 ** 8589934593))))))))",
    ),
    "1RB 0RD  0LC 1RE  1RA 1LD  0LC 1LC  0RD 0LE": (
        711,
        "(-3 + (3 * (2 ** (3 * (2 ** (-1 + (3 * (2 ** (-1 + (3 * (2 ** 6143)))))))))))",
    ),
    "1RB 2LA 1RA 2RB  2LB 1LA 3RB 1LB": (
        1480,
        "(2 + (2 ** (-9 + (2 ** (-7 + (2 ** (-7 + (2 ** (~10^36)))))))))",
    ),
    "1RB 1RE  1LC 1LF  1RD 0LB  1LE 0RC  1RA 0LD  1R_ 1LC": (
        10105,
        "((~10^15) + ((316 * (8 ** (2 + ((136081828 + (((~10^16) * (8 ** 5040)) + ((-557255229440 + (33215 * (8 ** 5040))) // 4095))) // 9)))) + ((-34828451840 + (132860 * (8 ** (2 + ((136081828 + (((~10^16) * (8 ** 5040)) + ((-557255229440 + (33215 * (8 ** 5040))) // 4095))) // 9))))) // 4095)))",
    ),
    "1RB 1LE  1RC 1RE  1RD 1RB  0RE 0LD  1LA 0LA": (
        1224,
        "(-4 + (3 * (2 ** (3 * (2 ** (-1 + (3 * (2 ** (-1 + (3 * (2 ** (-1 + (3 * (2 ** (-1 + (3 * (2 ** (-1 + (3 * (2 ** (-1 + (3 * (2 ** (-1 + (3 * (2 ** (~10^29)))))))))))))))))))))))))))",
    ),
    "1RB 0RD  1LC 1LE  1RD 1LB  0LD 1RA  0LA 1LC": (
        2621,
        "(-1 + (2 ** (-1 + (2 ** (2 ** (2 ** (2 ** (2 ** (2 ** (2 ** (2 ** (2 ** (2 ** (2 ** (2 ** (2 ** (2 ** (2 ** (2 ** (2 ** (2 ** 65536)))))))))))))))))))))",
    ),
    "1RB 0LD  0RC 0RA  1LD 1LE  1RE 1LC  0LE 1LA": (
        2447,
        "(23540 + (-(2 ** 11760) + (-(2 ** 17639) + ((2 ** 20578) * (-1 + ((~10^440) * (2 ** 1469)))))))",
    ),
    "1RB 1RC  1LC 1RA  0RC 1RD  1LE 0RB  0LA 0LD": (
        2661,
        "(23540 + (-(2 ** 11760) + (-(2 ** 17639) + ((2 ** 20578) * (-1 + ((~10^441) * (2 ** 1469)))))))",
    ),
    "1RB 1RA  1LC 0RB  1LB 1LD  0RA 0RE  0RB 1RE": (
        389,
        "(-1 + (3 * (2 ** (-3 + (3 * (2 ** (-2 + (3 * (2 ** (-2 + (3 * (2 ** (~10^14)))))))))))))",
    ),
    "1RB 2LA 1RA 4LA 5RA 0LB  1LA 3RA 2RB 1R_ 3RB 4LA": (
        5708,
        "(14 + (2 ** (-1 + (2 ** (2 + (2 ** (1 + (2 ** (-1 + (2 ** (4 + (2 ** (2 ** (-1 + (2 ** (2 ** (2 + (2 ** (2 ** (2 + (2 ** (2 ** (-1 + (2 ** (2 ** (-1 + (2 ** (2 ** (3 + (2 ** (2 ** (3 + (2 ** (2 ** (2 + (2 ** (2 ** (-1 + (2 ** (2 ** (-1 + (2 ** (2 ** (2 + (2 ** (2 ** (4 + (2 ** (2 ** (-1 + (2 ** (2 ** (2 + (2 ** (2 ** (2 + (2 ** (2 ** (-1 + (2 ** (2 ** (3 + (2 ** (2 ** (-1 + (2 ** (4 + (2 ** (2 ** (-1 + (2 ** (2 ** (-1 + (2 ** (2 ** (-1 + (2 ** (2 ** (4 + (2 ** (2 ** (-1 + (2 ** (2 ** (-1 + (2 ** (2 ** (1 + (2 ** (1 + (2 ** (-1 + (2 ** (4 + (2 ** (2 ** (-1 + (2 ** (2 ** (2 + (2 ** (2 ** (2 + (2 ** (2 ** (-1 + (2 ** (~10^78))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))",
    ),
    "1RB 3LA 4LB 0RB 1RA 3LA  2LA 2RA 4LA 1RA 5RB 1R_": (
        5524,
        "(1524 + ((157 * (2 ** (4 + ((5416 + ((83285 * (2 ** (~10^32))) + ((-81920 + (5 * (2 ** (~10^32)))) // 15))) // 3)))) + ((-21760 + (85 * (2 ** (4 + ((5416 + ((83285 * (2 ** (~10^32))) + ((-81920 + (5 * (2 ** (~10^32)))) // 15))) // 3))))) // 15)))",
    ),
    "1RB 1LE  1RC 1RE  1RD 1RB  0LB 0LD  1LA 0LA": (
        1080,
        "(-2 + (7 * (2 ** (-3 + (7 * (2 ** (-3 + (7 * (2 ** (-3 + (7 * (2 ** (-3 + (7 * (2 ** (-3 + (7 * (2 ** (-3 + (7 * (2 ** (-3 + (7 * (2 ** (-3 + (7 * (2 ** (~10^17))))))))))))))))))))))))))))",
    ),
    "1RB 2LA 3LA 2LA  3LB 3RA 0RA 0RB": (
        9652,
        "((~10^63) + ((2 ** (~10^63)) * (2 + ((2 ** (~10^63)) * (2 + ((2 ** (~10^63)) * (2 + ((2 ** (~10^63)) * (2 + (2 ** (~10^63)))))))))))",
    ),
    "1RB 2LA 0RB 0LA  3LB 2LA 1RA 3RA": (
        9210,
        "((~10^136) + ((2 ** (~10^136)) * (4 + ((2 ** (~10^136)) * (4 + ((2 ** (~10^136)) * (4 + ((2 ** (~10^136)) * (4 + (2 ** (~10^136)))))))))))",
    ),
    # "1RB 0RA  1LC 0LB  0LE 0LD  1LB 0LC  1RE 1RA": (3615, "(???)"),
    "1RB 0RE  1LC 1RE  1RD 0LB  1RA 0RD  1LC 0RA": (
        6225,
        "(729280535 + (-(2 ** 182320131) + (-(2 ** 273480197) + (-(2 ** 319060231) + (-(2 ** 341850249) + (-(2 ** 353245259) + (-(2 ** 358942765) + (-(2 ** 361791519) + (-(2 ** 363215897) + (-(2 ** 363928087) + (-(2 ** 364284183) + (-(2 ** 364462232) + (-(2 ** 364551257) + (-(2 ** 364595770) + (-(2 ** 364618028) + (-(2 ** 364629158) + (-(2 ** 364634724) + ((2 ** 364637508) * (-1 + ((~10^425) * (2 ** 1393)))))))))))))))))))))",
    ),
}

## test program ######################################################

PROGS: dict[
    str,
    tuple[
        set[str],
        set[int],
        set[str],
        set[int],
        str | None,
        tuple[str, ...],
    ],
] = {
    "1RB ...  1LB 0RC  1LC 1LA": (
        {'A', 'B', 'C'},
        {0, 1},
        {'A', 'B', 'C'},
        {0, 1},
        'A1',
        ('A0', 'A1', 'B0', 'B1', 'C0', 'C1'),
    ),
    "1RB ...  1RC ...  ... ...  ... ...  ... ...": (
        {'B', 'C'},
        {1},
        {'A', 'B', 'C', 'D'},
        {0, 1},
        None,
        ('A0', 'A1', 'B0', 'B1', 'C0', 'C1', 'D0', 'D1', 'E0', 'E1'),
    ),
    "1RB ... ... ...  1LB 1LA ... ...": (
        {'A', 'B'},
        {1},
        {'A', 'B'},
        {0, 1, 2},
        None,
        ('A0', 'A1', 'A2', 'A3', 'B0', 'B1', 'B2', 'B3')
    )
}

BRANCH = {
    ("1RB ...  ... ...", 'B0'): {
        '1RB ...  0LA ...',
        '1RB ...  0LB ...',
        '1RB ...  0RA ...',
        '1RB ...  0RB ...',
        '1RB ...  1LA ...',
        '1RB ...  1LB ...',
        '1RB ...  1RA ...',
        '1RB ...  1RB ...',
    },

    ("1RB 1LB  1LB 1LA", 'A1'): {
        '1RB 0LA  1LB 1LA',
        '1RB 0LB  1LB 1LA',
        '1RB 0RA  1LB 1LA',
        '1RB 0RB  1LB 1LA',
        '1RB 1LA  1LB 1LA',
        # '1RB 1LB  1LB 1LA',
        # '1RB 1RA  1LB 1LA',
        # '1RB 1RB  1LB 1LA',
    },

    ("1RB ...  1LC ...  ... ...  ... ...", 'D0'): {
        '1RB ...  1LC ...  ... ...  0LA ...',
        '1RB ...  1LC ...  ... ...  0LB ...',
        '1RB ...  1LC ...  ... ...  0LC ...',
        '1RB ...  1LC ...  ... ...  0LD ...',
        '1RB ...  1LC ...  ... ...  0RA ...',
        '1RB ...  1LC ...  ... ...  0RB ...',
        '1RB ...  1LC ...  ... ...  0RC ...',
        '1RB ...  1LC ...  ... ...  0RD ...',
        '1RB ...  1LC ...  ... ...  1LA ...',
        '1RB ...  1LC ...  ... ...  1LB ...',
        '1RB ...  1LC ...  ... ...  1LC ...',
        '1RB ...  1LC ...  ... ...  1LD ...',
        '1RB ...  1LC ...  ... ...  1RA ...',
        '1RB ...  1LC ...  ... ...  1RB ...',
        '1RB ...  1LC ...  ... ...  1RC ...',
        '1RB ...  1LC ...  ... ...  1RD ...',
    },

    ("1RB ... ... ...  1LB 1LA ... ...", 'A1'): {
        '1RB 0LA ... ...  1LB 1LA ... ...',
        '1RB 0LB ... ...  1LB 1LA ... ...',
        '1RB 0RA ... ...  1LB 1LA ... ...',
        '1RB 0RB ... ...  1LB 1LA ... ...',
        '1RB 1LA ... ...  1LB 1LA ... ...',
        '1RB 1LB ... ...  1LB 1LA ... ...',
        '1RB 1RA ... ...  1LB 1LA ... ...',
        '1RB 1RB ... ...  1LB 1LA ... ...',
        '1RB 2LA ... ...  1LB 1LA ... ...',
        '1RB 2LB ... ...  1LB 1LA ... ...',
        '1RB 2RA ... ...  1LB 1LA ... ...',
        '1RB 2RB ... ...  1LB 1LA ... ...',
    },

    ("1RB ... ... ...  ... ... ... ...", 'B0'): {
        '1RB ... ... ...  0LA ... ... ...',
        '1RB ... ... ...  0LB ... ... ...',
        '1RB ... ... ...  0RA ... ... ...',
        '1RB ... ... ...  0RB ... ... ...',
        '1RB ... ... ...  1LA ... ... ...',
        '1RB ... ... ...  1LB ... ... ...',
        '1RB ... ... ...  1RA ... ... ...',
        '1RB ... ... ...  1RB ... ... ...',
        '1RB ... ... ...  2LA ... ... ...',
        '1RB ... ... ...  2LB ... ... ...',
        '1RB ... ... ...  2RA ... ... ...',
        '1RB ... ... ...  2RB ... ... ...',
    },
}

NORMALIZE = {
    '1RB 2LA 1RA 1LA  3LA 1R_ 2RB 2RA': {
        '1RB 3LA 1LA 1RA  2LA 1R_ 3RA 3RB',
        '2RB 2RA 1LA 2LA  3LA 1RB 2R_ 1RA',
        '1LB 2RA 1LA 1RA  3RA 1L_ 2LB 2LA',
        '1LB 3RA 1RA 1LA  2RA 1L_ 3LA 3LB',
        '2LB 2LA 1RA 2RA  3RA 1LB 2L_ 1LA',
    },
    '1RB 1LC  1RD 1RB  0RE 1RE  1LD 1LA  0LF 1LF  0RD 0RC': {
        '1RB 1LE  1RD 1RB  0RD 0RE  1LD 1LA  0RF 1RF  0LC 1LC',
        '1RB 1LD  1RE 1RB  0RE 0RD  0RF 1RF  1LE 1LA  0LC 1LC',
        '1LB 1RC  1LD 1LB  0LE 1LE  1RD 1RA  0RF 1RF  0LD 0LC',
        '1LB 1RE  1LD 1LB  0LD 0LE  1RD 1RA  0LF 1LF  0RC 1RC',
        '1LB 1RD  1LE 1LB  0LE 0LD  0LF 1LF  1RE 1RA  0RC 1RC',
    },
}

## test graph ########################################################

A, B, C, D, E = "A", "B", "C", "D", "E"

# flat, norm, conn, irr, zrefl, entries, exits

GRAPHS: dict[
    str,
    tuple[
        str,
        int, int, int, int,
        dict[str, set[str]],
        dict[str, set[str]],
    ],
] = {
    # 2 2
    "1RB 1LB  1LA 1R_": (
        "BBA_",
        1, 1, 1, 0,
        {A: {B}, B: {A}},
        {A: {B}, B: {A}},
    ),
    "1RB 1LB  1LB 1LA": (
        "BBBA",
        1, 1, 0, 1,
        {A: {B}, B: {A, B}},
        {A: {B}, B: {A, B}},
    ),

    # 3 2
    "1RB 0LB  1LA 0RA  ... ...": (
        "BBAA..",
        0, 0, 1, 0,
        {A: {B}, B: {A}, C: set()},
        {A: {B}, B: {A}, C: set()},
    ),
    "1RB 1LA  0LA 0RB  ... ...": (
        "BAAB..",
        0, 0, 0, 0,
        {A : {A, B}, B: {A, B}, C: set()},
        {A : {A, B}, B: {A, B}, C: set()},
    ),
    "1RB ...  0LB 1RC  1LB 0RC": (
        "B.BCBC",
        1, 0, 0, 1,
        {A: set(), B: {A, B, C}, C: {B, C}},
        {A: {B}, B: {B, C}, C: {B, C}},
    ),
    "1RB 1R_  1LB 0RC  1LC 1LA": (
        "B_BCCA",
        1, 1, 0, 1,
        {A: {C}, B: {A, B}, C: {B, C}},
        {A: {B}, B: {B, C}, C: {A, C}},
    ),
    "1RC 1R_  1LB 0RC  1LB 1LA": (
        "C_BCBA",
        0, 1, 0, 1,
        {A: {C}, B: {B, C}, C: {A, B}},
        {A: {C}, B: {B, C}, C: {A, B}},
    ),
    "1RB 0LB  1LA 0RC  1LC 1LA": (
        "BBACCA",
        1, 1, 0, 1,
        {A: {B, C}, B: {A}, C: {B, C}},
        {A: {B}, B: {A, C}, C: {A, C}},
    ),
    "1RB 0LA  1LB 0RC  1LC 1LB": (
        "BABCCB",
        1, 0, 0, 1,
        {A: {A}, B: {A, B, C}, C: {B, C}},
        {A: {A, B}, B: {B, C}, C: {B, C}},
    ),

    # 2 3
    "1RB 2LB 1R_  2LA 2RB 1LB": (
        "BB_ABB",
        1, 1, 0, 0,
        {A: {B}, B: {A, B}},
        {A: {B}, B: {A, B}},
    ),
    "1RB 2LB 1LA  2LB 2RA 0RA": (
        "BBABAA",
        1, 1, 0, 1,
        {A: {A, B}, B: {A, B}},
        {A: {A, B}, B: {A, B}},
    ),
    "1RB 2LB 1LA  2LB 2RB 0RB": (
        "BBABBB",
        1, 0, 0, 1,
        {A: {A}, B: {A, B}},
        {A: {A, B}, B: {B}},
    ),
    "1RB 0RA 2LB  2LA 0LA 1RA": (
        "BABAAA",
        1, 1, 0, 0,
        {A: {A, B}, B: {A}},
        {A: {A, B}, B: {A}},
    ),

    # 4 2
    "1RB 1LB  1LA 0LC  1R_ 1LD  1RD 0RA": (
        "BBAC_DDA",
        1, 1, 0, 1,
        {A: {B, D}, B: {A}, C: {B}, D: {C, D}},
        {A: {B}, B: {A, C}, C: {D}, D: {A, D}},
    ),
    "1RC 1LB  1LA 0LC  1R_ 1LD  1RC 0RA": (
        "CBAC_DCA",
        0, 1, 1, 0,
        {A: {B, D}, B: {A}, C: {A, B, D}, D: {C}},
        {A: {B, C}, B: {A, C}, C: {D}, D: {A, C}},
    ),
    "1RB 1LB  1LA 0LB  1R_ 1LC  1RD 0RA": (
        "BBAB_CDA",
        1, 0, 0, 1,
        {A: {B, D}, B: {A, B}, C: {C}, D: {D}},
        {A: {B}, B: {A, B}, C: {C}, D: {A, D}},
    ),
    "1RC 1LB  1LA 0LC  1R_ 1LD  1RD 0RD": (
        "CBAC_DDD",
        0, 0, 0, 1,
        {A: {B}, B: {A}, C: {A, B}, D: {C, D}},
        {A: {B, C}, B: {A, C}, C: {D}, D: {D}},
    ),
    "1RB 0LC  1LD 0LA  1RC 1RD  1LA 0LD": (
        "BCDACDAD",
        1, 1, 0, 1,
        {A: {B, D}, B: {A}, C: {A, C}, D: {B, C, D}},
        {A: {B, C}, B: {A, D}, C: {C, D}, D: {A, D}},
    ),
    "1RB 0LC  1LC 0LA  1RC 1RB  1LA 0LD": (
        "BCCACBAD",
        1, 0, 0, 1,
        {A: {B, D}, B: {A, C}, C: {A, B, C}, D: {D}},
        {A: {B, C}, B: {A, C}, C: {B, C}, D: {A, D}},
    ),
    "1RC 0LB  1LD 0LA  1RC 1RD  1LA 0LD": (
        "CBDACDAD",
        0, 1, 0, 1,
        {A: {B, D}, B: {A}, C: {A, C}, D: {B, C, D}},
        {A: {B, C}, B: {A, D}, C: {C, D}, D: {A, D}},
    ),
    "1RC 0LC  1LD 0LA  1RC 1RD  1LA 0LD": (
        "CCDACDAD",
        0, 0, 0, 1,
        {A: {B, D}, B: set(), C: {A, C}, D: {B, C, D}},
        {A: {C}, B: {A, D}, C: {C, D}, D: {A, D}},
    ),

    # 2 4
    "1RB 2LA 1RA 1RA  1LB 1LA 3RB 1R_": (
        "BAAABAB_",
        1, 1, 0, 1,
        {A: {A, B}, B: {A, B}},
        {A: {A, B}, B: {A, B}},
    ),
    "1RA 2LA 1RA 1RA  1LB 1LA 3RB 1R_": (
        "AAAABAB_",
        1, 0, 0, 1,
        {A: {A, B}, B: {B}},
        {A: {A}, B: {A, B}},
    ),

    # 3 3
    "1RB 2LB 1LC  1LA 2RB 1RB  1R_ 2LA 0LC": (
        "BBCABB_AC",
        1, 1, 0, 0,
        {A: {B, C}, B: {A, B}, C: {A, C}},
        {A: {B, C}, B: {A, B}, C: {A, C}},
    ),
    "1RB 2LB 1LA  1LA 2RB 1RB  1R_ 2LA 0LC": (
        "BBAABB_AC",
        1, 0, 0, 0,
        {A: {A, B, C}, B: {A, B}, C: {C}},
        {A: {A, B}, B: {A, B}, C: {A, C}},
    ),
    "1RC 2LB 1LC  1LA 2RB 1RB  1R_ 2LA 0LC": (
        "CBCABB_AC",
        0, 1, 0, 0,
        {A: {B, C}, B: {A, B}, C: {A, C}},
        {A: {B, C}, B: {A, B}, C: {A, C}},
    ),
    "1RB 1LC 1R_  1LA 1LC 2RB  1RB 2LC 1RC": (
        "BC_ACBBCC",
        1, 1, 0, 0,
        {A: {B}, B: {A, B, C}, C: {A, B, C}},
        {A: {B, C}, B: {A, B, C}, C: {B, C}},
    ),

    # 5 2
    "1RB 1LC  1RC 1RB  1RD 0LE  1LA 1LD  1R_ 0LA": (
        "BCCBDEAD_A",
        1, 1, 0, 0,
        {A: {D, E}, B: {A, B}, C: {A, B}, D: {C, D}, E: {C}},
        {A: {B, C}, B: {B, C}, C: {D, E}, D: {A, D}, E: {A}},
    ),
    "1RB 1LC  1RC 1RB  0LE 1RD  1LA 1LD  1R_ 0LA": (
        "BCCBEDAD_A",
        0, 1, 0, 0,
        {A: {D, E}, B: {A, B}, C: {A, B}, D: {C, D}, E: {C}},
        {A: {B, C}, B: {B, C}, C: {E, D}, D: {A, D}, E: {A}},
    ),
    "1RB 1LC  1RC 1RB  1RD 0LC  1LA 1LD  1R_ 0LE": (
        "BCCBDCAD_E",
        1, 0, 0, 0,
        {A: {D}, B: {A, B}, C: {A, B, C}, D: {C, D}, E: {E}},
        {A: {B, C}, B: {B, C}, C: {C, D}, D: {A, D}, E: {E}},
    ),
    "1RB 1LC  1LC 1RA  1LB 0LD  1LA 0RE  1RD 1RE": (
        "BCCABDAEDE",
        1, 1, 0, 0,
        {A: {B, D}, B: {A, C}, C: {A, B}, D: {C, E}, E: {D, E}},
        {A: {B, C}, B: {A, C}, C: {B, D}, D: {A, E}, E: {D, E}},
    ),
}
