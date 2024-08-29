# pylint: disable = line-too-long, too-many-lines, consider-using-namedtuple-or-dataclass

## test turing #######################################################

BasicTermData = dict[str, tuple[int, int]]

HALT: BasicTermData = {
    # 2/2 BB
    "1RB 1LB  1LA ...": (4, 6),
    "1RB 0LB  1LA ...": (3, 6),
    "1RB ...  1LB 1LA": (3, 6),
    "1RB ...  0LB 1LA": (2, 6),

    # 3/2 BB
    "1RB ...  1LB 0RC  1LC 1LA": (5, 21),  # shift
    "1RB ...  0LC 0RC  1LC 1LA": (5, 20),
    "1RB 1LA  0RC ...  1LC 0LA": (5, 20),
    "1RB 1RA  0RC ...  1LC 0LA": (5, 19),
    "1RB 0RA  0RC ...  1LC 0LA": (4, 19),
    "1RB 1LC  1RC ...  1LA 0LB": (6, 11),  # sigma

    # 2/3 BB
    "1RB 2LB ...  2LA 2RB 1LB": (9, 38),
    "1RB 0LB ...  2LA 1RB 1RA": (8, 29),
    "0LB 2RB ...  1RA 1LB 1LA": (6, 27),
    "1RB 1LA 1LB  0LA 2RA ...": (6, 26),
    "1RB 2LA ...  1LB 1LA 0RA": (6, 26),

    # 4/2 BB
    "1RB 1LB  1LA 0LC  ... 1LD  1RD 0RA": (13, 107),  # shift
    "1RB 1LC  1LD 0RB  ... 0LD  1RA 1LA": ( 9,  97),
    "1RB 0RC  1LA 1RA  ... 1RD  1LD 0LB": (13,  96),  # sigma
    "1RB 1LB  0LC 0RD  ... 1LA  1RA 0LA": ( 6,  96),
    "1RB 1LC  0LD 0RD  ... 0LA  1LD 1LA": (11,  84),

    # 2/4
    "1RB 2LA 1RA 1RA  1LB 1LA 3RB ...": (2050, 3932964),  # BB
    "1RB 2LA 1RA 1LA  3LA ... 2RB 2RA": (  90,    7195),
    "1RB 2LA 1RA 1LA  3LA ... 2RB 2LA": (  84,    6445),
    "1RB 2LA 1RA 1LA  3LA ... 2RB 3RA": (  84,    6445),
    "1RB 2RB 3LA 2RA  1LA 3RB ... 1LB": (  60,    2351),

    # 5/2 BB
    "1RB 1LC  1RC 1RB  1RD 0LE  1LA 1LD  ... 0LA": (4098, 47176870),

    # 5/5 block-compiled from 5/2
    "1RB ... ... ... ...  2LC ... ... ... ...  3RD 3LC ... 1LC ...  ... 1RD 1RB 1LE ...  4RD 1LE ... 1RD 1LC": (4097, 15721561),

    # 3/3
    "1RB 2RA 2RC  1LC ... 1LA  1RA 2LB 1LC": (36089, 310341163),
    "1RB ... 2RB  1LC 0LB 1RA  1RA 2LC 1RC": (107900, 4939345068),

    # Copy of 2/4 BB
    "1RB 1LC ...  1LA 1LC 2RB  1RB 2LC 1RC": (2050, 3932964),
    # Surprise-in-a-box
    "1RB 2LB 1LC  1LA 2RB 1RB  ... 2LA 0LC": (31, 2315619),
    # R. Blodgett
    "1RB 1LB 2LB  1RC 1LA 0RA  1LA 2RA ...": (9, 57),
    # David Jefferson
    "1RB 1RA ...  1LC 1LC 2LA  2RA 1LB 1LA": (12, 44),

    # Milton Green (1964)
    # "1RB ...  ... ...": (1, 2),
    "1RB ...  0RC 1RC  0RD 0RC  1RE 1LA  0RF 0RE  1LF 1LD": (35, 436),
    "1RB 1RC  0RD 0RB  ... 1RA  1RE 1LF  0RG 0RE  0RC 1RB  1LG 1LD": (22961, 197_700_005),

    # Lynn (1971)
    "1RB 1RA  1LC 0LD  0RA 1LB  ... 0LE  1RC 1RB": (15, 435),
    "1RB 1RC  1LC 1LD  0RA 1LB  1RE 0LB  ... 1RD": (22, 292),
    "1RB 0RC  1LC 0LB  1RD 1LB  1RE 0RA  0RB ...": (22, 217),
    # Lynn reports 522 steps
    "1RB 0LB  1LC ...  0LD 0LC  1LE 0RA  0LF 0LE  1RF 1RD": (42, 521),

    # Uwe (1981)

    # Castor diligentissimus et primus et perpetuus (Castor schultis)
    "1RB 0LC  1RC 1RD  1LA 0RB  0RE ...  1LC 1RA": (501, 134467),

    # Castor ministerialis: the Civil Servant Beaver, who
    # cares most for his progress, but does not produce anything.
    "1RB 1RA  1RC 0RD  1LE 0RA  ... 0RB  1LB 1LE": (0, 52),

    # Castor scientificus: the Scientific Beaver, who does
    # not produce anything either, but with more effort and
    # less effect on his position.
    "0RB 0LA  0RC ...  1RD 1LE  1LA 0LD  1RC 1RE": (0, 187),

    # Castor exflippus: the Beaver Freak, who tries to
    # survive as long as possible without producing
    # anything, moving on the tape, and changing his state.
    "0RB 0LA  1RC ...  0LC 1RD  0LD 1RE  1LA 0LE": (0, 67),
}

HALT_SLOW = {
    # 3/3
    "1RB 2LA 1RA  1RC 2RB 0RC  1LA ... 1LA": (1525688, 987522842126),
    "1RB ... 2LC  1LC 2RB 1LB  1LA 2RC 2LA": (2950149, 4144465135614),
    "1RB 2RC 1LA  2LA 1RB ...  2RB 2RA 1LC": (95524079, 0),
    "1RB 2LA 1LC  0LA 2RB 1LB  ... 1RA 1RC": (374676383, 0),
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

    # 7/4 block-compiled from 4/2 BBB sigma
    "1RB ... ... ...  0LC 2LC ... ...  0LC 3RD 0RD 2RE  1LF 1LC 1RB ...  ... 3RD ... ...  1LG ... 2RB 1LF  2RE ... 2LC ...": (35, 1434),

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

    # 10/2 backsym-compile from 5/2 spaghetti
    "1RB 1RC  1LD ...  0RE 0LF  0LF 1LD  1LF ...  1RG 0LF  1RG 1RA": (1, 10675),
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
    "1RB 0LA  0RC 0RD  1LA 0RA  0LC 1RA": ( 22269896, 2353542),
    "1RB 0LA  0RC 1LA  1RD 0RD  1LB 1RB": ( 24378294, 2575984),
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
    "1RB 1LB  1LA 0LC  ... 1LD  1RD 0RA": {
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
    "1RB 2LA 1RA 1RA  1LB 1LA 3RB ...": {
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
    "1RB 1LC  1RC 1RB  1RD 0LE  1LA 1LD  ... 0LA": {
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
    "1RB 0LC  1RC 1RD  1LA 0RB  0RE ...  1LC 1RA": {
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

MOTHER = {
    "1RB 1LE  0LC 0LB  0LD 1LC  1RD 1RA  ... 0LA",
}

BIGFOOT = {
    "1RB 2RA 1LC  2LC 1RB 2RB  ... 2LA 1LA",
    "1RB 1RC  1RE 1RE  1LD 0RA  1RB 1LG  1LG 1RF  0RE 1RE  1LH 0LH  ... 1LC",
}

HYDRA = {
    "1RB 3RB ... 3LA 1RA  2LA 3RA 4LB 0LB 0LA",
    "1RB 3RB ... 3LA 1RA  2LA 3RA 4LB 0LB 1LB",
}

ANTIHYDRA = {
    "1RB 1RA  0LC 1LE  1LD 1LC  1LA 0LB  1LF 1RE  ... 0RA",
}

CRYPTIDS = MOTHER | BIGFOOT | HYDRA | ANTIHYDRA

SPAGHETTI = {
    # Halt
    "1RB 2RA 2RC  1LC ... 1LA  1RA 2LB 1LC",  # 310341163

    "1RB 1LA 3LA 3RC  2LC 2LB 1RB 1RA  2LA 3LC ... 1LB",
    "1RB 3LA 3RC 1RA  2RC 1LA ... 2RB  1LC 1RB 1LB 2RA",

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
    "1RB 0LC  1RC 1RD  1LA 0RB  0RE ...  1LC 1RA": 3,  # 134467 Uwe
    "1RB 0LC  1RC 1RD  1LA 0RB  0RE ...  ... 1RA": 3,  # partial

    "1RB 1LA 1RD  2LC 0RA 1LB  2LA 0LB 0RD  2RC ... 0LC": 3,
    "1RB 1LD ...  1RC 2LB 2LD  1LC 2RA 0RD  1RC 1LA 0LA": 3,

    "1RB 1RA 1LB 1RC  2LA 0LB 3LC ...  1LB 0RC 2RA 2RC": 3,

    "1RB 0LD  1RC 0RF  1LC 1LA  0LE ...  1LF 0RB  0RC 0RE": 4,  # Pavel, BB6
    "1RB 1RC  1LC 0RF  1RA 0LD  0LC 0LE  1LD 0RA  1RE ...": 4,  # Shawn
    "1RB 0LD  1RC 0RF  1LC 1LA  0LE ...  1LA 0RB  0RC 0RE": 3,  # Pavel
    "1RB 0LC  1LA 1RD  1RA 0LE  1RA 0RB  1LF 1LC  1RD ...": 3,
    "1RB 0LC  1LA 1RD  0LB 0LE  1RA 0RB  1LF 1LC  1RD ...": 3,
    "1RB 0RC  0LA 0RD  1RD ...  1LE 0LD  1RF 1LB  1RA 1RE": 3,
    "1RB 1RF  0RC 0RA  1LD ...  0LE 0LB  1LF 1LD  1RA 1LB": 3,
    "1RB 0LF  1RC 0LE  1RD 0RA  1LE ...  1RC 0LA  1RA 0LB": 3,

    "1RB 2LC 3LA 0RD  3LE 2RB 0RB 1LE  1RD 2LA ... ...  1RF 0LA 1RD 2LA  2RF 3RF 1LG ...  1LA 0RF 3LA 0RD  2RB 3RB 1LE 1LH  3LE 3LH ... 1RB": 3,

    # Spinout
    "1RB 1RC 0RC  1RC 0LA 1LB  2LC 2RA 1LB": 3,

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
    "1RB 0LD  1LC 0RA  1LA 1LB  1LD 0LC": 3,
    "1RB 1RC  1LC 0LD  1RA 0LB  ... 0LA": 3,
    "1RB 1RC  1LC 0RD  1RA 0LB  1RD 0RA": 3,

    # Quasihalt
    "1RB 0LC  1RC 1LD  1RD 0RB  0LB 1LA": 3,
    "1RB 1LC  1LC 0RD  1LA 0LB  1LD 0RA": 3,
    "1RB 0RC  0RD 1RA  0LD 0LA  1LC 1LA": 3,

    "1RB 1LC  1LC 1RA  1LB 0LD  1LA 0RE  1RD 1RE": 3,  # 221032, 2
    "1RB 1LC  1RD 1RA  1LB 0LA  1RE 0RC  1RC 0LE": 3,

    # Quasihalt Fixed
    "1RB 0LC  1LD 0RC  1RA 0RB  0LD 1LA": 3,  # 1459, 1
    "1RB 1RC  1LD 0RA  0RC 1RD  1RA 0LB": 3,

    "1RB 0RD  1LC 0RA  1LA 1LB  ... 0RC": 3,

    "1RB 1LC 2LC 2RA  2LD 3RE 1RB ...  0LC 2RA 2LF 3LD  ... 1LF 1RE 3LC  1LC 3LC 3RE 2RG  1RB 1RE 2LC 3RE  0LD 2RA 1RB 3LD": 6,

    "1RB ... ... ...  2LC 3RD ... ...  1LA 3RD 1LE 4RD  ... ... 1RF ...  1RF 2LG 2LE 2RH  3RI 2RH 3RJ ...  1LE ... ... 2LC  2LE 2RK 2RH ...  1LE ... ... ...  0RI 1RF 0RJ ...  2RB ... 2RF ...": 5,

    # infrul
    "1RB 0LC  1RC 0LD  1LD 0RB  0LA 0LA": 3,
    "1RB 0LD  0LC 1RA  1LA 1LD  1LB 0RA": 3,
    "1RB 0LD  0LC 1RA  1RA 1LD  1LB 0RA": 3,
    "1RB 0LD  0LC 1RA  1RC 1LD  1LB 0RA": 3,
    "1RB 0LD  0RC 1LD  1RD 1RA  1LB 0RA": 3,
    "1RB 0RB  1RC 0LD  1LD 0LB  0RA 1LC": 3,
    "1RB 0RC  1RC 1LD  1LB 0LD  0RA 0LB": 3,
    "1RB 1LC  0LA 1RC  1LA 0RD  0LA 0RB": 3,
    "1RB 1LC  1LA 0LC  0LD 1RD  0RB 0RA": 3,
    "1RB 1LC  1LA 1RC  0LD 0RB  0RB 0LA": 3,
    "1RB 1LC  1LA 1RC  0LD 0RB  1LB 0LA": 3,
    "1RB 1LC  1LA 1RC  0LD 0RB  1RA 0LA": 3,
    "1RB 1LC  1LA 1RC  0LD 0RB  1RD 0LA": 3,
    "1RB 1LC  1LA 1RC  0RD 0LA  1RA 0RB": 3,

    "1RB 1LA 0RC  2LB 2RC 1LA  0LA 0RB 0LB": 3,

    "1RB 0RE  0LC 1RD  1LC 1LD  1RE 0RA  0LA 0RB": 3,
    "1RB 0RE  1RC 1RB  1LD 1LE  0LD 0RA  1LB 1LD": 3,
    "1RB 1LC  0LA 0RD  1LA 1RB  0LE 1RC  1LE 0RB": 3,
    "1RB 0RD  1LC 0RC  0LE 1RD  1RA 0RB  1LE 1LA": 3,
    "1RB 0LC  1LC 0LA  0RA 0LD  0RE 1LB  1RE 1RB": 3,
    "1RB 1LD  0RC 1LA  0LD 1RC  0RE 0LA  1LA 0RB": 3,
    "1RB 0LD  1RC 0LA  1LD 0RA  1LE 0RA  0RD 1LC": 3,
    "1RB 0LD  1RC 0LA  1LD 0RA  1LE 0RA  0RC 1LC": 3,
    "1RB 0LC  1LC 0RD  0RE 1LA  0LA 1RD  0RB 1LB": 3,
    "1RB ...  0RC 0RE  1LD 1RE  0LD 1RA  1LB 0LC": 3,
    "1RB 0RD  1RC 1LE  0LA ...  1LD 0RE  1LA 0RB": 3,
    "1RB 0RC  1LA 1LD  0LC 1RD  0RE 0LA  0RB ...": 3,
    "1RB 0RD  0LC 1RD  1LC 1LA  0LE 0RB  ... 0RC": 3,
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

    # incomplete
    "1RB ...  1LB 0RB",
    "1RB ... ...  2LB 1RB 1LB",
    "1RB ...  1LB 0LC  1RC 1RB",
    "1RB ...  1LB 1LC  1RC 0RB",
    "1RB ...  1LB 1RC  0LC 0RB",
    "1RB ...  1LC 0LC  1RC 0LB",
    "1RB ...  1LC 0RB  1LB 1RC",
    "1RB ...  0LC 0LB  1RC 1RD  0LE 1RB  1LB 1LE",
    "1RB ...  0LC 0LB  1RC 1RD  1LE 1RB  0LC 1LE",
    "1RB ...  0LC 1LB  1RC 1RD  1LB 1RE  0LC 0LE",
    "1RB ...  0LC 1RD  1LD 1LC  0LE 0LD  1RE 1RB",
    "1RB ...  0RC 0RB  1LC 1LD  0RE 1LB  1LB 1RE",
    "1RB ...  0RC 0RB  1LC 1LD  1RE 1LB  0LD 1RE",
    "1RB ...  0RC 0RB  1LC 1LD  1RE 1LB  1RC 1RE",
    "1RB ...  0RC 1LD  0RD 1RC  0RE 0RD  1LE 1LB",
    "1RB ...  0RC 1LD  1LB 1RC  0RE 0RD  1LE 1LB",
    "1RB ...  0RC 1LD  1LD 1RC  0RE 0RD  1LE 1LB",
    "1RB ...  0RC 1RB  1LC 1LD  1RE 0RD  0LE 0RB",
    "1RB ...  1LB 0LC  1LD 1RC  1RE 1LE  0RB 0RE",
    "1RB ...  1LB 1LC  0LD 1LE  1RD 0LE  0RB 0RE",
    "1RB ...  1LB 1LC  0RD 1LE  0RE 1RD  0RB 0RE",
    "1RB ...  1LB 1LC  1RD 1LE  0LE 1RD  0RB 0RE",
    "1RB ...  1LB 1LC  1RD 1LE  1LE 0RC  0RB 0RE",
    "1RB ...  1LC 1LB  0LD 0LC  1RD 1RE  0LB 1RC",
    "1RB ...  1LC 1RB  1RD 1LD  0RE 0RD  1LE 0LB",
    "1RB ...  1LC 1RD  0LE 1LC  0LE 0LD  1RE 1RB",
    "1RB ...  1RC 1LB  1LD 1RD  0LE 0LD  1RE 0RB",
    "1RB ...  1RC 1LD  0RE 1RC  0RE 0RD  1LE 1LB",
    "1RB ...  1RC 1RB  0RD 0RC  1LD 1LE  0RB 1LC",
    "1RB ...  1RC 1RB  1LC 1LD  1RB 1LE  0RC 0RE",
    "1RB ...  1RC ...  1LC 1LD  1RE 1LF  1RC 1RE  0RC 0RF",
    "1RB ... ... ... ...  2LC ... ... ... ...  3RD 3LC ... 1LC ...  ... 1RD 1RB 1LE ...  4RD 1LE ... 1RD 1LC",
    "1RB ... ... ...  0LC 2LC ... ...  0LC 3RD 0RD 2RE  1LF 1LC 1RB ...  ... 3RD ... ...  1LG ... 2RB 1LF  2RE ... 2LC ...",
}

CANT_REACH_STEPS: dict[str, dict[str, int]] = {
    "halt": {
        "1RB 0RB  0RC 1LD  1LD ...  0LD 0RA": 40,
        "1RB 0LA  0RC 1RC  1RD 1LA  1LA ...": 40,
        "1RB 0RB  1RC 1LD  1LA ...  0LD 0RA": 46,
        "1RB 0LA  0RC 1RC  1RD 1LA  1LB ...": 46,

        "1RB ...  1RC 1LC  1LD 0RE  0RA 0LD  0LB 1RE": 105,
        "1RB 0LA  0RC 1RC  1RD 1LA  1RE 1LB  1LB ...": 107,
        "1RB 0LA  0RC 1RC  1RD 1LA  0RE 1LB  1LC ...": 115,
        "1RB 0RB  1RC 1LE  1RD 1LA  1LB ...  0LE 0RA": 115,
    },

    "spinout": {
        "1RB 0RC  1LB 1RC  0LD 0RB  0RA 1LD": 25,
        "1RB 0RB  1LC 0LD  1LB 1RA  0RD 0LC": 29,

        "1RB 0LB 2RC  1LA 1LC 2RC  1RC 2LA 0LA": 103,
        "1RB 1RC 0RC  1LC 1RC 2LA  2RC 2LB 0LB": 125,

        "1RB 0RC  1RC 0RA  1LB 0RD  0LE 1RD  1LE 1LB": 102,
        "1RB 0LC  1LC 0LA  0RA 0LD  0RE 1LB  1RE 1RB": 108,
        "1RB 0RE  1RC 1RB  1LD 1LE  0LD 0RA  1LB 1LD": 112,
        "1RB 0RE  0LC 1RD  1LC 1LD  1RE 0RA  0LA 0RB": 120,
        "1RB 0LE  0RC 1LB  1RD 0RD  0LA 1RC  1LE 0LB": 145,
        "1RB 0RE  0LC 1RA  1LC 1LD  1RE 0RE  0LB 0RB": 147,
        "1RB 0RD  1LC 0RC  0LE 1RD  1RA 0RB  1LE 1LA": 149,
        "1RB 1RC  1LC 0RB  0RC 0LD  1LA 0LE  0LA ...": 190,
        "1RB 1LC  0LA 0RD  1LA 1RB  0LE 1RC  1LE 0RB": 191,
        "1RB 1RE  1LC 0LA  1LA 0RD  0LB 1LB  0RE 0LD": 211,
        "1RB 0RE  1RC 1LC  1LD 0LB  0RA 0LC  1RE 0LD": 223,
        "1RB 0RB  0LC 1RD  1LA 1LC  0LE 1RA  1LE 0RD": 256,
    },

    "blank": {
        "1RB 2LA 3LA 0RB  0LA 2RB 1LB 3RB": 30,
        "1RB 0RB 1RA 3LA  2LB 2RB 3RB 0LA": 34,

        "1RB 0RC  1LB 0LC  0RD 0LD  1RA 0LA": 23,
        "1RB 1LC  1LA 1RC  0LD 0RB  0RB 0LA": 40,
        "1RB 0LB  1RC 0LD  1LA 0RB  0RC 1LD": 41,
        "1RB 0LC  1RC 0LD  1LA 0RB  0RC 1LD": 41,
        "1RB 1LC  1RC 0LD  1LA 0RB  0RC 1LD": 41,
        "1RB 0LD  1LC 0RA  1RA 0LB  0RB 1LD": 41,
        "1RB 0LC  1RC 0LD  1LD 0RB  0LA 0LA": 44,

        "1RB 0RB 2LC  1LC 2RB 0LC  1RA 1LA 0LA": 164,
        "1RB 1LB 0LB  1RC 0RC 2LA  1LA 2RC 0LA": 164,
        "1RB 0RB 1LA  2LC 2RA 0LA  1RA 1LC 0RA": 183,
        "1RB 1LA 0RB  1RC 0RC 1LB  2LA 2RB 0LB": 183,
        "1RB 0RC 0LC  1LB 2RB 0LA  1LA 1RA 2LA": 184,
        "1RB 1LB 2RB  1LC 0LA 0RA  1RC 2LC 0RB": 184,
        "1RB 1LA 0RC  2LB 2RC 1LA  0LA 0RB 0LB": 188,
        "1RB 1RC 2LA  1LA 0RC 0LA  2RC 2RB 0LB": 266,

        "1RB 3LB 2LA 3LA 0RB  2LB 3RB 0LA 4RB 4RA": 154,
        "1RB 1RA 0RB 4LA 4LB  2LB 2RB 3RB 2LA 0LA": 163,
        "1RB 2RA 0RB 4LA 3LB  2LB 2RB 3RB 2LA 0LA": 170,
        "1RB 2RB 0RB 2RA 4LA  1LA 3LB 3RB 4RB 0LA": 243,
        "1RB 3RA 1LA 4LA 0RB  2LA 3LB 0LA 2LA 4RB": 247,
        "1RB 3RA 3LA 4LA 0RB  2LA 2LA 0LA 1LB 4RB": 247,
        "1RB 0RB 1RB 2RA 4LA  2LA 3RB 3LB 4RB 0LA": 247,
        "1RB 0RB 3RA 1RB 4LA  2LA 2RB 3LB 4RB 0LA": 247,

        "1RB 0RB  0LC 1RD  1LA 1LC  0LE 1RA  1LE 0RD": 254,
        "1RB 0LD  0RC 1RC  1LA 0LE  0RA 1LD  0LA 1RB": 271,
        "1RB 1LD  0RC 1LA  0LD 1RC  0RE 0LA  1LA 0RB": 276,
        "1RB 0RB  1RC 0LD  0LD 1RB  0RA 1LE  1LD 1LA": 283,
        "1RB 0RB  1RC 0LD  1LC 1RB  0RA 1LE  1LD 1LA": 295,
        "1RB 0LD  1RC 1RA  0LA 1RB  0RA 1LE  1LD 0RA": 303,
        "1RB 0LD  1RC 1RA  0LA 0LB  0RA 1LE  1LD 0RA": 317,
        "1RB 0LB  1LC 0RD  1LA 1LB  0RE 1RD  1RC 0LC": 321,
        "1RB 0LB  1LC 1LD  1RD 0LD  1LB 0RE  0RA 1RE": 321,
        "1RB 0LD  1RC 1RA  1LA 0RA  0LE 1LD  1LB 0RB": 321,
        "1RB 1RC  1LC 0RC  1RA 0LD  0LE 1LD  1LA 0RA": 321,
        "1RB 0LD  1RC 0LA  1LD 0RA  1LE 0RA  0RC 1LC": 413,
        "1RB 0LD  1RC 0LA  1LD 0RA  1LE 0RA  0RD 1LC": 413,
        "1RB 0LD  1RC 0LA  1LD 0RA  1LE 1RE  0RC 1LC": 433,
        "1RB 0RD  1LC 0LA  0RA 1LB  1RE 0LB  0LB 1RD": 583,
        "1RB 0RD  1LC 0LA  0RA 1LB  1RE 0LB  1LE 1RD": 659,
        "1RB 0LC  1LC 0RD  0RE 1LA  0LA 1RD  0RB 1LB": 796,
    },
}

CANT_HALT_FALSE_NEGATIVES: set[str] = {
    "1RB 0RA  1LA ...",

    "1RB ...  0LC 0RB  1LC 1LA",
    "1RB ...  1LC 1RA  1LA 0LC",
    "1RB ...  1LC 0RC  1RA 0LC",
    "1RB ...  1LB 0LC  1LA 1RA",
    "1RB ...  0LC 0RA  0RA 1LB",
    "1RB ...  0LC 0RA  1LA 1LB",
    "1RB ...  1LC 0RA  0RB 0LB",
    "1RB ...  1LC 0RA  1RB 0LB",
    "1RB ...  1LC 0RB  1LB 1LA",
    "1RB ...  1LC 1RA  0RA 0LC",
    "1RB 0LB  1LA 0RC  1RB ...",
    "1RB 0LB  1LC 1RB  ... 1LA",
    "1RB 0LC  0LC 1RA  ... 1LA",
    "1RB 0LC  1LA 0RA  1LA ...",
    "1RB 1LC  0LA 0RB  1LA ...",
    "1RB 1RC  1LC ...  0RA 0LB",

    "1RB 2LB ...  1LB 2LA 1RB",
    "1RB 0LA ...  1LB 2LA 0RB",
    "1RB 2LA 0RB  2LA ... 1RA",
    "1RB ... 0RB  2LB 2RA 0LA",
    "1RB ... 0RB  2LB 2LA 0RA",
    "1RB 0LB ...  1LA 2RA 1LA",
    "1RB 0LB ...  2LA ... 0RA",
    "1RB 0LB ...  2LA 2LA 1RA",
    "1RB 1LA 0LA  0LA 2RA ...",
    "1RB 1LA 0LA  1LA 2RA ...",
    "1RB 1LA 0LA  2LA 0RA ...",
    "1RB 1LA 0LA  2LA 2RA ...",
    "1RB 1LB ...  2LA 0RB 1RB",
    "1RB 2LA 0LA  0LA ... 2RA",
    "1RB 2LA 0LA  1LA ... 2RA",
    "1RB 2LA 0LA  2LA ... 2RA",
    "1RB 2LA 0RB  0LA ... 1RA",
    "1RB 2LA 1RB  0LA ... 1RA",
    "1RB 2LA 1RB  1LA ... 1RA",
    "1RB 2LA 1RB  2LA ... 1RA",
    "1RB 2LB ...  1LA 1RB 0RB",
    "1RB 2LB ...  2LA 2LA 1RB",
    "1RB 2LB 1RB  1LA 0RA ...",
    "1RB 2LB 1RB  2LA ... 0RA",
    "1RB 2RB 1LA  0LA 2RA ...",

    "1RB 1LA ... 3LA  2LA 3RB 3LA 0RA",
    "1RB 3LA 1LA ...  1LB 2RB 0LA 0RB",
    "1RB 3LA 1LA 1RA  2LB 2RA ... 0RB",
    "1RB 3LA 1LA 2RA  2LB 3RA ... 0RB",

    "1RB 1RC  1LC 0LD  1RA 0LB  ... 0LA",
    "1RB ...  1LC 0RB  0LC 0LD  0RD 1RA",
    "1RB ...  1LC 1RD  1LA 0LC  0RD 0LA",
    "1RB 0LD  1LC 0RB  0LC 1LD  0RA ...",
    "1RB 1LC  0LA ...  1RD 0LC  0RD 0RB",
    "1RB 1LA  0RC 0RB  1LC 1LD  0LA ...",
    "1RB 1LD  1RC 0RB  1LA ...  0LD 1RB",
    "1RB 1LC  0LA ...  1RD 0LC  0RD 1RB",
    "1RB ...  0LC 1RC  0RC 1LD  1LA 0LD",
    "1RB 1LB  0LB 1RC  1RD 0RC  1LA ...",
    "1RB 1RD  0LB 1LC  0RA ...  1LB 0RD",
    "1RB ...  1LC 0RB  0LC 0LD  0RA ...",
    "1RB ...  1LC 1RC  0RC 1LD  1LA 0LD",
    "1RB 1LA  0RC 0RB  0LC 1LD  0LA ...",
    "1RB 1RB  1LC 0RB  0LC 1LD  0RA ...",
    "1RB 1LD  1LC 0RB  0LC 0RA  1LA ...",
    "1RB ...  1LC 0RB  0LC 0LD  1RD 1RA",
    "1RB 0LA  1LC ...  0LD 0LC  1RD 0RA",
    "1RB 0RD  1LC 0RA  1LA 1LB  ... 0RC",

    "1RB ...  0LC 0LB  1RC 1RD  1LE 1RB  1LA 1LE",
    "1RB 1LC  1RD 1RB  0RE 0RC  0RC ...  1LE 1LA",
    "1RB 1RA  1RC ...  0RD 0RC  1LD 1LE  1RA 1LC",
    "1RB ...  1RC 1RB  0RD 0RC  1LD 1LE  1LA 1LC",
    "1RB 1RC  1LD ...  0LE 0LC  0LC 1LD  1RE 1RA",
    "1RB 1LA  0LC 0LB  1RC 1RD  1RE 1RB  1LA ...",
    "1RB 1RA  1LC ...  1RA 1LD  0RE 0RD  1LE 1LC",
    "1RB ...  1LC 1RB  1LA 1LD  0RE 0RD  1LE 1LC",
    "1RB 1RA  1RC ...  1LC 1LD  0RA 1LE  0RC 0RE",
    "1RB 1RC  1LD ...  0LE 0LC  1RE 1LD  1RE 1RA",
    "1RB 1RC  0LD ...  0LE 0LC  1LE 1LD  1RE 1RA",
    "1RB 1LA  0LA 1RC  1RA 1RD  ... 0RE  0LA 1RC",
    "1RB 1LC  0LA 0RD  1LA 1LE  1RB 1LE  0LC ...",

    "1RB 2RB 3RB 4RB 5LA 4RA  0LA 1RB 5RA ... ... 1LB",

    "1RB 0RC  0RC 0LD  1RD 1RA  1RE 0LA  1LF ...  1LB 1LF",
    "1RB 0RF  1LC 0LC  1LE 0RD  1RA 1LB  0LD 0LE  ... 0RD",
    "1RB ...  1RC 0RF  1RD 0LF  1LE 0RC  1LD 0RE  1LC 1RA",
    "1RB 0LA  1RC 0RF  1RD 0RA  1LE 0RD  1LB 1LE  ... 1RD",
    "1RB 0RC  1LC 0LF  1LE 0RD  1RA 1LB  0LD 0LE  ... 0RD",
    "1RB 0RC  1LA 0RE  1RA 1LD  0LC 0LA  0RD 0RF  0RE ...",
    "1RB ...  1LC 0RB  1LD 0RD  0RE 1LB  0RC 1RF  0RA 1RE",
    "1RB 0RD  0RC 0LD  1RD ...  1LE 1LF  1RB 1LD  1RF 0RA",
    "1RB 0RC  1LC 0LC  1LE 0RD  1RA 1LB  0LD 0LF  0LD ...",
    "1RB 1LC  1LA 0LD  1RB 0LA  ... 1LE  1RF 0LB  1RB 0RE",
    "1RB 0LD  1RC 1RF  0LA 1LF  ... 1LE  0LF 0RF  1RA 1LC",
    "1RB 0LD  1RC 1RF  0LA 1LF  ... 1LE  0LF 0LA  1RA 1LC",
    "1RB 0RC  1RC 1RA  1RD 1LA  1LE 0RF  0LA 1LE  ... 0RD",

    "1RB 1RC  1LD ...  0RE 0LF  0LF 1LD  1LF ...  1RG 0LF  1RG 1RA",

    "1RB ... ... ...  0LC 2LC ... ...  0LC 3RD 0RD 2RE  1LF 1LC 1RB ...  ... 3RD ... ...  1LG ... 2RB 1LF  2RE ... 2LC ...",
    "1RB 1LC 2LC 2RA  2LD 3RE 1RB ...  0LC 2RA 2LF 3LD  ... 1LF 1RE 3LC  1LC 3LC 3RE 2RG  1RB 1RE 2LC 3RE  0LD 2RA 1RB 3LD",
}

CANT_BLANK_FALSE_NEGATIVES: set[str] = {
    "1RB 1LB  1LA 0RB",
    "1RB 0LB  1LA 0RB",
    "1RB 1LA  0LA 0RB",

    "1RB ...  0LC 0RB  1LC 1LA",
    "1RB 0RA  0RC ...  1LC 0LA",
    "1RB ...  1LC 0RB  1LB 1RC",
    "1RB ...  1LC 0RC  1RA 0LC",
    "1RB ...  1LC 1RA  1LA 0LC",
    "1RB ...  1LB 1RC  0LC 0RB",
    "1RB 1LB  0RC 1RC  1LA 0LA",
    "1RB 0LB  1LC 0RC  1RA 1LA",
    "1RB 1LB  0RC 0RB  1LC 0LA",
    "1RB 0LA  0RC 1LA  1LC 0RB",
    "1RB 0LA  1LB 0RC  1LC 1LA",
    "1RB 0LA  0RC 0RC  1LC 1LA",
    "1RB 1LA  0LB 1RC  0LA 0RC",
    "1RB 1LA  0LA 1RC  0LA 0RC",
    "1RB 0RB  0RC 0RA  1LC 1RB",
    "1RB 0LB  0LC 0LA  1RC 1LB",
    "1RB 0RB  1LC 0RA  1LC 1LA",
    "1RB 0RC  1LB 1RA  0RB 0RA",
    "1RB ...  1LC 0RB  1LB 1LA",
    "1RB ...  1LC 1RA  0RA 0LC",
    "1RB 1LC  0LA 0RB  1LA ...",

    "1RB 2LA 0RB  0LB 1LA 0RA",
    "1RB 2LA 1LB  0LA 0RB 1RA",
    "1RB 2LA 0LA  1LA 2RA 1RB",
    "1RB 2LB 1LA  1LA 2RB 0RB",
    "1RB 2LA 0LA  1LA 2RA 0RB",
    "1RB 2LA 1LA  0LA 0RB 2RB",
    "1RB 1LA 2LA  0LA 2RB 0RB",
    "1RB 2LB 0LA  1LA 2RB 0RB",
    "1RB 0LA ...  1LB 2LA 0RB",
    "1RB 2LB 2LA  2LA 0LB 0RA",
    "1RB 0RA 2LB  2LA 2RA 0LB",
    "1RB 2LA 2LB  1LA 2RA 0LB",
    "1RB 1LA 2LB  2LA 2RA 0LB",
    "1RB 1LA 2LB  1LA 2RA 0LB",
    "1RB 0RA 1LB  2LA 2RB 0LA",
    "1RB 0RA 2LB  2LA 0LA 1RA",
    "1RB 2LB 0RA  1LA 2RB 2RA",
    "1RB 2LB 0RA  1LA 1RB 2RA",
    "1RB 2RB 2LA  0LA 0RB 1LA",
    "1RB 2RB 1LA  0LA 2LA 0RB",
    "1RB 2LA 0RB  2LB 1LA 0RA",
    "1RB ... 0RB  2LB 2LA 0RA",
    "1RB 2LB ...  1LA 1RB 0RB",
    "1RB 2LA 0LA  2LA ... 2RA",
    "1RB 1LA 0LA  0LA 2RA ...",
    "1RB 1LA 0LA  2LA 2RA ...",
    "1RB 1LA 0LA  2LA 0RA ...",
    "1RB 2LA 0LA  1LA ... 2RA",
    "1RB 1LA 0LA  1LA 2RA ...",
    "1RB 1LB ...  2LA 0RB 1RB",
    "1RB 2LA 0LA  0LA ... 2RA",

    "1RB 1LC  1LD 0RB  ... 0LD  1RA 1LA",
    "1RB 0LA  1LC 1LD  1RD 1LB  1RA 0RD",
    "1RB 0LC  1RD 1LD  0LA 1LB  1LC 0RD",
    "1RB 0RA  1RC 0RB  1LD 1LC  1RA 0LC",
    "1RB 1RA  0RC 0LB  0RD 0RA  1LD 0LA",
    "1RB 1LA  0RC 1RC  1LD 0RB  0LD 1LA",
    "1RB 0LA  0RC 0RD  1LC 1LA  0RB 1RD",
    "1RB 0LC  0RD 1RD  1LA 1LC  1RC 0RB",
    "1RB 0LC  0RD 1RD  0LA 1LC  1LA 0RA",
    "1RB 0LA  1LC 0RA  0LD 1RD  1LA 0RB",
    "1RB 0LC  1RC 0RD  1LA 1LC  1RA 0RB",
    "1RB 1RC  1LC 0RB  1LD 0RA  1RA 0LB",
    "1RB 0LA  0RC 1RD  1LD 0RB  1LA 1RD",
    "1RB 0LC  1RD 0RD  1LA 0RC  1LB 1RC",
    "1RB 0LC  1RC 1RD  1LD 0RC  1LA 0RB",
    "1RB 1LC  1RD 0RB  0LC 1LA  1RC 0RA",
    "1RB 1RC  1LB 1LC  1RD 0LB  1RA 0RD",
    "1RB 0LC  1RD 0RB  1LC 1LA  1RC 1RA",
    "1RB 1LA  1RC 0RD  0LA 0RC  1RC 1LC",
    "1RB 1LA  1RC 1LD  1RD 0RC  0LD 0LA",
    "1RB 1LA  1LC 0RA  1LD 0LC  1RB 0LA",
    "1RB 0LC  0RD 0RC  1LD 0RB  1LA 0LC",
    "1RB 0LC  1RD 0RA  0LB 0LA  1LC 0RA",
    "1RB 1RC  0RC 1RA  1LD 0RB  0LD 1LA",
    "1RB 1RA  1LC 0RB  1RC 0LD  1LA 1LD",
    "1RB 1LA  1LC 0RA  1LD 0LC  1RA 0LA",
    "1RB 0RA  1LC 1RA  1LD 0LC  1LA 0RB",
    "1RB 0LC  0RC 1LD  1RD 0LA  1LB 1LA",
    "1RB 0LA  0RC 1LA  1RD 1RC  1LD 1LB",
    "1RB 0LC  1LD 1LC  1RD 0LA  0RA 1LB",
    "1RB 1LA  1LB 0RC  1LC 1LD  0RA 0LD",
    "1RB 0LA  1LB 0RC  1RD 1RC  1LA 1LD",
    "1RB 0RA  1RC 0LB  1LD 0RD  1RA 1LB",
    "1RB 0RA  1RC 0LD  0LB 1RA  0LA 1LD",
    "1RB 0RC  1LB 1RC  1RA 0LD  1LA 1LC",
    "1RB 1RC  1LC 0RB  1RA 0LD  0LC 1LD",
    "1RB 0LC  1RD 1LC  0LA 1LB  1LC 0RD",
    "1RB 1RC  1LC 1RD  1RA 1LD  0RD 0LB",
    "1RB 0LC  1LD 0RC  1RA 0RB  0LD 1LA",
    "1RB 0LB  1LB 1LC  1RD 0LB  1RA 0RD",
    "1RB 0LC  0RD 1LC  0LA 1LB  1LD 0RB",
    "1RB 1RC  1LD 0RA  0RC 1RD  1RA 0LB",
    "1RB 1RC  1RD 0LC  1LD 0LD  1LB 0RA",
    "1RB 1LC  1LD 0RA  1RC 0LD  0LC 1LA",
    "1RB 0LC  0RD 1RC  1LA 1RD  1LD 0RB",
    "1RB 1LA  1RC 1LD  1RD 0RC  1LB 0LA",
    "1RB 0RA  1LC 1RD  1LD 0LC  0RB 0LA",
    "1RB 1LC  1LC 0RB  0LD 1LA  1RA 0LC",
    "1RB 0LB  1RC 0RB  0LD 1RA  0LA 1LD",
    "1RB 0RA  0LC 1RD  0LD 1LC  1RA 0LA",
    "1RB 1RA  1LC 0LD  1RA 0LB  0LB 0RA",
    "1RB 0RC  1LD 0RA  0RA 0LD  1LA 1LD",
    "1RB 0LC  1RC 1RB  1LA 0LD  0LC 0RB",
    "1RB 1LC  1LD 0LC  0LB 0RA  1RD 1RA",
    "1RB 0RC  1LB 1LD  0RA 0LD  1LA 1RC",
    "1RB 1LB  1RC 0LD  0LA 1RA  0LB 0RA",
    "1RB 0LC  0LD 1RD  0LA 0RD  1RA 1LA",
    "1RB 0LA  1RC 0RA  1LD 0RC  0LB 1LA",
    "1RB 0RC  1LD 0RB  1RA 0LC  0LA 1LC",
    "1RB 0LA  0RC 1RD  1LA 0LD  1LC 0RD",
    "1RB 0LA  0RC 0RD  1LA 0RA  0LC 1RA",
    "1RB 0LA  0RC 1LA  1RD 0RD  1LB 1RB",
    "1RB 1LB  0LB 1RC  1RD 0RC  1LA ...",
    "1RB 0LA  1LC ...  0LD 0LC  1RD 0RA",
    "1RB 1LA  0LA 1LC  0LD 0LC  1RD 1RB",

    "1RB 3LA 1LA ...  1LB 2RB 0LA 0RB",
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
    "1RB 2LB 0LA 1LA  2LA 3RA 1RB 0LB",
    "1RB 2LA 3RB 2RB  3LA 3RA 0LB 1RA",
    "1RB 2RB 3LB 0RA  1LA 3RB 2LA 2RA",
    "1RB 0RB 0LA 2LB  1LB 2LA 3RB 1RA",
    "1RB 0LA 0RB 2LB  3LB 3RA 0RA 1LA",
    "1RB 0LA 0RB 2LB  3LB 3RA 1RB 1LA",
    "1RB 2RA 2LA 3LB  0LB 1LA 3RB 0RA",
    "1RB 2RA 3LA 0LB  1LB 1LA 0RB 1RB",
    "1RB 2LB 2RA 3LA  1LA 3RA 3LB 0LB",
    "1RB 2LB 0LA 1LB  3LA 0RA 3RA 2RB",
    "1RB 2LB 3RB 0LB  1LA 2RA 0LB 2LB",
    "1RB 2RA 3LA 0LA  1LB 2LA 0RB 3RA",
    "1RB 2LB 3RB 1LA  2LA 2LA 3RA 0RB",
    "1RB 2LB 0LA 1RB  3LA 2LA 3RB 1RA",
    "1RB 2LA 3RB 0LA  2LB 1RA 2RA 1LA",
    "1RB 2LA 3LB 0RA  0LA 3LB 1RA 2RB",
    "1RB 1LA 0RB 2LA  3LA 3RA 0LB 1RA",
    "1RB 2LB 1LB 0RA  2LA 3RB 2RB 0LA",
    "1RB 1LB 0RA 2RB  1LA 2RA 3LB 3LA",
    "1RB 2LB 3RA 3RB  1LA 1RA 0LB 2LA",
    "1RB 2LA 0LB 1RA  3LA 2RA 0RA 0LB",
    "1RB 3LA 1LA 1RA  2LB 2RA ... 0RB",
    "1RB 3LA 1LA 2RA  2LB 3RA ... 0RB",

    "1RB ... 2RB  1LC 0LB 1RA  1RA 2LC 1RC",
    "1RB 2LB 1LC  1LA 2RB 1RB  ... 2LA 0LC",
    "1RB 2RA 1LC  2LB 0RB 2LA  1LA 0LB 1LA",
    "1RB 2RA 1LC  2LB 0RB 2LA  2RB 0LB 1LA",
    "1RB 2RC 1LA  2LA 1RB 0RB  2RB 2RA 1LC",

    "1RB 2LB 4LB 3LA ...  1LA 3RA 3LB 0LB 0RA",

    "1RB 0LA  0RC 1LA  1RD 1RE  1RE ...  1LB 1LA",
    "1RB 0RD  0LC 1RE  1RA 1LD  0LC 1LC  0LD 0LE",
    "1RB 0RC  1LC 0LD  1RE 0LD  0LC 1LB  0RE 1RA",
    "1RB 0LE  1RC 1LB  0RD 0RB  1LD 0LE  0RC 1LA",
    "1RB 1RC  0LC 1RD  1LB 1LE  1RD 0RA  1LA 0LE",
    "1RB 0RC  1LC 0LB  1RD 1LB  1RE 0RA  0RB ...",
    "1RB 1LE  1RC 0RA  0LD 0LC  1RD 1RE  1LB 0RB",
    "1RB 0RE  1RC 0RB  1LD 0RA  0LA 0LC  0LE 1LA",
    "1RB 0RE  0LC 0LB  1RC 1RD  1LA 0RA  1RA 1LD",
    "1RB 0LE  0RC 0LC  0RD 1RA  1LD 1LA  1LC 0RB",
    "1RB 0RD  0LC 1RE  1RA 1LD  0LC 1LC  0RD 0LE",
    "1RB 1LE  1RC 1RE  1RD 1RB  0LB 0LD  1LA 0LA",
    "1RB 0LE  0RC 1RB  0RD 1RA  1LD 1LA  1LC 0RB",
    "1RB 1LA  1RC 0RE  1LD 0LA  1LC 0RD  1RC 1RB",
    "1RB 1RA  1LC 1RE  0RE 1LD  1RE 1LC  1LA 0LE",
    "1RB 1LE  1RC 1RE  1RD 1RB  0RE 0LD  1LA 0LA",
    "1RB 0RA  1LC 0LB  0LE 0LD  1LB 0LC  1RE 1RA",
    "1RB 1RA  0LB 1RC  0LD 0LC  1RA 1LE  0LC 1LD",
    "1RB 1RA  1LC 0RB  1LB 1LD  0RA 0RE  0RB 1RE",
    "1RB 0RD  0RC 0RB  1LC 0LA  0RA 1RE  0LB 1RB",
    "1RB 1RA  0LC 1RE  0LE 1LD  0LB 1LC  1LA 0LE",
    "1RB 0RA  1RC 1RE  1LD 0LA  1LC 0RD  0RB 1RB",
    "1RB 0LD  1RC 1LB  1LA 1RE  1LE 1LA  1RC 0RA",
    "1RB 0LA  1RC 1LD  1LB 0RE  1LB 0LB  1LE 0RA",
    "1RB 0LA  1RC ...  0RD 1LE  1LC 1RD  0RA 1LA",
    "1RB 0LA  0LC 0RD  0RA 1LB  1RC 1LE  0LE 1LC",

    "1RB 1RA 2LB 3LA  2LA 0LB 1LC 1LB  3RB 3RC ... 1LC",
    "1RB 1RA 1LB 1RC  2LA 0LB 3LC ...  1LB 0RC 2RA 2RC",
    "1RB 1LA 1LB 1RA  0LA 2RB 2LC ...  3RB 2LB 1RC 0RC",

    "1RB 2LD ...  2LC 2RC 2RB  1LD 0RC 1RC  2LA 2LD 0LB",
    "1RB 0LB 1RD  2RC 2LA 0LA  1LB 0LA 0LA  1RA 0RA ...",
    "1RB 1LA 1RD  2LC 0RA 1LB  2LA 0LB 0RD  2RC ... 0LC",

    "1RB 0LF  1RC 0LE  1RD 0RA  1LE ...  1RC 0LA  1RA 0LB",
    "1RB 0LD  1RC 0RF  1LC 1LA  0LE ...  1LF 0RB  0RC 0RE",
    "1RB ...  0RC 1RC  0RD 0RC  1RE 1LA  0RF 0RE  1LF 1LD",
    "1RB 0LA  1LC 1LF  0LD 0LC  0LE 0LB  1RE 0RA  ... 1LD",
    "1RB 0LB  1LC ...  0LD 0LC  1LE 0RA  0LF 0LE  1RF 1RD",
    "1RB 1RC  1LC 0RF  1RA 0LD  0LC 0LE  1LD 0RA  1RE ...",
    "1RB 0LD  1RC 0RF  1LC 1LA  0LE ...  1LA 0RB  0RC 0RE",
    "1RB 0RF  0LB 1LC  1LD 0RC  1LE ...  1LF 0LD  1RA 0LE",
    "1RB 0LF  0RC 0RD  1LD 1RE  0LE 0LD  0RA 1RC  1LA ...",
    "1RB 0RF  1RC 1RA  1RD 0RC  1LE 0RB  1LB 0LD  ... 0LD",
    "1RB 0LC  1LA 1RC  1RA 0LD  1LE 1LC  1RF ...  1RA 1RE",
    "1RB 0LC  1LA 1LD  1RD 0RC  0LB 0RE  1RC 1LF  1LE ...",
    "1RB 0LC  1LA 1RD  1RA 0LE  1RA 0RB  1LF 1LC  1RD ...",
    "1RB 0RC  0RC 0LD  1RD 1RA  1RE 0LA  1LF ...  1LB 1LF",
    "1RB 0RF  1LC 0LC  1LE 0RD  1RA 1LB  0LD 0LE  ... 0RD",
    "1RB 0LA  1RC 0RF  1RD 0RA  1LE 0RD  1LB 1LE  ... 1RD",
    "1RB 0RC  1LC 0LF  1LE 0RD  1RA 1LB  0LD 0LE  ... 0RD",
    "1RB 1LC  0RC 1RE  1LD 1RA  0RE 0LF  1RC 0RD  1LE 1LD",
    "1RB 0RC  1RC 1RA  1RD 1LA  1LE 0RF  0LA 1LE  ... 0RD",
    "1RB 0RC  1LA 0RE  1RA 1LD  0LC 0LA  0RD 0RF  0RE ...",
    "1RB ...  1LC 0RB  1LD 0RD  0RE 1LB  0RC 1RF  0RA 1RE",

    "1RB 1RC  0RD 0RB  ... 1RA  1RE 1LF  0RG 0RE  0RC 1RB  1LG 1LD",

    "1RB 1RC  1RE 1RE  1LD 0RA  1RB 1LG  1LG 1RF  0RE 1RE  1LH 0LH  0LG 1LC",
    "1RB 1RC  1RE 1RE  1LD 0RA  1RB 1LG  1LG 1RF  0RE 1RE  1LH 0LH  0RG 1LC",
    "1LB ...  0LC 1LC  0LD 0LC  1LE 1RA  0LF 0LE  1LG 1RD  0LH 0LG  1RH 1RF",

    "1RB 1RC  0RD 0RB  ... 1RA  1RE 1LF  0RG 0RE  0RC 1RB  1RH 1LD  0RI 0RH  1LI 1LG",

    "1RB ... ... ...  0LC 2LC ... ...  0LC 3RD 0RD 2RE  1LF 1LC 1RB ...  ... 3RD ... ...  1LG ... 2RB 1LF  2RE ... 2LC ...",

    "1RB 2LC 3LA 0RD  3LE 2RB 0RB 1LE  1RD 2LA ... ...  1RF 0LA 1RD 2LA  2RF 3RF 1LG ...  1LA 0RF 3LA 0RD  2RB 3RB 1LE 1LH  3LE 3LH ... 1RB",

    "1RB ... ... ...  2LC 3RD ... ...  1LA 3RD 1LE 4RD  ... ... 1RF ...  1RF 2LG 2LE 2RH  3RI 2RH 3RJ ...  1LE ... ... 2LC  2LE 2RK 2RH ...  1LE ... ... ...  0RI 1RF 0RJ ...  2RB ... 2RF ...",
}

CANT_SPIN_OUT_FALSE_NEGATIVES: set[str] = {
    "1RB ...  1LB 1RC  0LC 0RB",
    "1RB 0LA  0RC 1LA  1LC 0RB",
    "1RB 1LB  0RC 0RB  1LC 0LA",
    "1RB ...  0LC 0RB  1LC 1LA",
    "1RB 0LB  0LC 0LA  1RC 1LB",
    "1RB 0LB  1LA 0RC  1LC 1RB",
    "1RB 0LC  1LA 0RA  1RC 1LA",
    "1RB 0RB  0RC 0RA  1LC 1RB",
    "1RB 0RB  1LC 0RA  1LC 1LA",
    "1RB 0RC  1LB 1RA  0RB 0RA",

    "1RB ... 0RB  2LB 2RA 0LA",
    "1RB 0LA ...  1LB 2LA 0RB",
    "1RB 0RB 0LB  1LB 2RA 1LA",
    "1RB 2LA 0RB  0LB 1LA 0RA",
    "1RB ... 0RB  2LB 2LA 0RA",
    "1RB 2LA 0RB  2LB 1LA 0RA",

    # slow
    "1RB ... ...  2LB 1RB 1LB",
    "1RB 1RA 0RB  2LB 1LA 1LB",

    "1RB 0LC  1RD 0RB  1LC 1LA  1RC 1RA",
    "1RB 1LC  1RD 0RB  0LC 1LA  1RC 0RA",
    "1RB 1RA  0RC 0LB  0RD 0RA  1LD 0LA",
    "1RB 1RC  1LB 1LC  1RD 0LB  1RA 0RD",
    "1RB 0LA  0RC 0RD  1LC 1LA  0RB 1RD",
    "1RB 1LA  0RC 1RC  1LD 0RB  0LD 1LA",
    "1RB 1RC  0RC 1RA  1LD 0RB  0LD 1LA",
    "1RB 1LC  1LD 0RA  1RC 0LD  0LC 1LA",
    "1RB 0LC  0RD 1RC  1LA 1RD  1LD 0RB",
    "1RB 0LA  1LC ...  0LD 0LC  1RD 0RA",
    "1RB 1LB  0LB 1RC  1RD 0RC  1LA ...",
    "1RB 1LA  0LA 1LC  0LD 0LC  1RD 1RB",
    "1RB 0LD  1LC 0RA  1LA 1LB  1LD 0LC",
    "1RB 1RC  1LC 0RD  1RA 0LB  1RD 0RA",

    "1RB 0LA 1RA 0LB  2LB 3LA 2RB 0RA",
    "1RB 0LA 2RB 0RB  3LB 2LA 1RA 1RA",
    "1RB 1RA 1LB 2RB  1LB 3LA 3LB 0RA",
    "1RB 2LA 0LA 0RB  3LB 1LA 3RA 0LA",
    "1RB 2LA 0LB 1RA  1LB 3LA 3RB 3RB",
    "1RB 2LA 0LB 1RA  3LB 2LA 3RA 0RA",
    "1RB 2RA 3LA 0LA  1LB 2LA 0RB 3RA",
    "1RB 2LA 1RA 2RB  2LB 1LA 3RB 1LB",
    "1RB 3LA 1LA 1RA  2LB 2RA 0RB 3RB",  # QH 77, xmas
    "1RB 2LA 2RB 1LA  3LB 3RA 2RB 0RB",  # QH 14, xmas
    "1RB 3LA 1LA ...  1LB 2RB 0LA 0RB",
    "1RB 3LA 1LA 1RA  2LB 2RA ... 0RB",
    "1RB 3LA 1LA 2RA  2LB 3RA ... 0RB",

    "1RB 3LA 1LA 4LA 1RA  2LB 2RA ... 0RA 0RB",
    "1RB 4LA 1LA ... 2RB  2LB 3LA 1LB 2RA 0RB",

    "1RB 0LE  0RC 0LC  0RD 1RA  1LD 1LA  1LC 0RB",
    "1RB 0LE  0RC 1RB  0RD 1RA  1LD 1LA  1LC 0RB",
    "1RB 0RD  0RC 0RB  1LC 0LA  0RA 1RE  0LB 1RB",
    "1RB 0RD  1LC 1LE  1RD 1LB  0LD 1RA  0LA 1LC",
    "1RB 0RA  1LC 0LB  0LE 0LD  1LB 0LC  1RE 1RA",
    "1RB 0LA  0LC 0RD  0RA 1LB  1RC 1LE  0LE 1LC",

    "1RB ...  0RC 1RC  0RD 0RC  1RE 1LA  0RF 0RE  1LF 1LD",
    "1RB 0LD  1RC 0RF  1LC 1LA  0LE ...  1LF 0RB  0RC 0RE",
    "1RB 0LA  1LC 1LF  0LD 0LC  0LE 0LB  1RE 0RA  ... 1LD",
    "1RB 0LB  1LC ...  0LD 0LC  1LE 0RA  0LF 0LE  1RF 1RD",
    "1RB 0LD  1RC 0RF  1LC 1LA  0LE ...  1LA 0RB  0RC 0RE",

    "1RB 1RC  0RD 0RA  1LE 0LF  0RC 0RA  1LE 0LF  0LG 0RD  0RA 1LE",
    "1RB 1RC  0RD 0RB  ... 1RA  1RE 1LF  0RG 0RE  0RC 1RB  1LG 1LD",

    "1LB ...  0LC 1LC  0LD 0LC  1LE 1RA  0LF 0LE  1LG 1RD  0LH 0LG  1RH 1RF",

    "1RB 1RC  0RD 0RB  ... 1RA  1RE 1LF  0RG 0RE  0RC 1RB  1RH 1LD  0RI 0RH  1LI 1LG",
}

DO_HALT: set[str] = {
    # 3/4
    "1RB 3LB ... 2RA  2LC 3RB 1LC 2RA  3RB 1LB 3LC 2RC",  # 2^[15]5 + 14
    "1RB 0LB ... 3LA  0LC 3RB 3RC 1LB  2RB 2LA 3RA 1LC",  # 10^^2048

    # 2/6
    "1RB 3RB 5RA 1LB 5LA 2LB  2LA 2RA 4RB ... 3LB 2LA",  # 10^^^3
    "1RB 3LA 4LB 0RB 1RA 3LA  2LA 2RA 4LA 1RA 5RB ...",  # 10^^90
    "1RB 2LA 1RA 4LA 5RA 0LB  1LA 3RA 2RB ... 3RB 4LA",  # 10^^70
    "1RB 2LA 5LB 0RA 1RA 3LB  1LA 4LA 3LB 3RB 3RB ...",  # 10^^24

    # 6/2
    "1RB 0LD  1RC 0RF  1LC 1LA  0LE ...  1LF 0RB  0RC 0RE",  # 10^^15
    "1RB 0LA  1LC 1LF  0LD 0LC  0LE 0LB  1RE 0RA  ... 1LD",  # 10^^5
    "1RB ...  0LC 0LD  1LD 1LC  1RE 1LB  1RF 1RD  0LD 0RA",  # 10^1292913985
    "1RB ...  1RC 1RA  1RD 0RB  1LE 0RC  0LF 0LD  0LB 1LA",  # 10^197282
    "1RB 1RC  1LC 0RF  1RA 0LD  0LC 0LE  1LD 0RA  1RE ...",  # 10^78913
    "1RB 1LE  1RC 1RF  1LD 0RB  1RE 0LC  1LA 0RD  ... 1RC",  # 10^36534
    "1RB 0LD  1RC 0RF  1LC 1LA  0LE ...  1LA 0RB  0RC 0RE",  # 10^21132
    "1RB 0LE  1LC 0RA  1LD 0RC  1LE 0LF  1LA 1LC  1LE ...",  # 10^2879
    "1RB 0RF  0LB 1LC  1LD 0RC  1LE ...  1LF 0LD  1RA 0LE",  # 10^1762
    "1RB 0LF  0RC 0RD  1LD 1RE  0LE 0LD  0RA 1RC  1LA ...",  # 10^1730

    # 2/5
    "1RB 3LA 4RB 0RB 2LA  1LB 2LA 3LA 1RA ...",  # ???
    "1RB 2LB 4LB 3LA ...  1LA 3RA 3LB 0LB 0RA",  # 10^19017
    "1RB 2LA 1RA 2LB 2LA  0LA 2RB 3RB 4RA ...",  # 10^704
    "1RB 2LA 4RA 2LB 2LA  0LA 2RB 3RB 4RA ...",  # 10^211
    "1RB 2LA 4RA 2LB 2LA  0LA 2RB 3RB 1RA ...",  # 10^211
    "1RB 2LA 1RA 2LB 3RA  0LA 2RB 4RB ... 2RA",  # 10^176
    "1RB 2LA 1RA 2LB 2RA  0LA 3RB 3RB 4RA ...",  # 10^113
    "1RB 2LA 1RA 2LB 3RA  0LA 2RB 4RB ... 2LA",  # 10^101
    "1RB 2LA 4RA 1LB 2LA  0LA 2RB 3RB 2RA ...",  # 10^61
    "1RB 2LA 1RA 2RB 2LB  0LA 3RA 4RB ... 1RA",  # 10^60
    "1RB 2LA 1RA 2LB 2RA  0LA 2RB 3RB 4RA ...",  # 10^52
    "1RB 4LA 1LA ... 2RB  2LB 3LA 1LB 2RA 0RB",  #      7,021,292,621
    "1RB 3LA 1LA 4LA 1RA  2LB 2RA ... 0RA 0RB",  # 26,375,397,569,930
}

DONT_BLANK: set[str] = {
    "1RB 2LA 1LA  2LA 2RB 0RA",  # wolfram

    "1RB 1LA 2LA  0LA 2RB 0RB",
    "1RB 2LA 0LA  1LA 2RA 0RB",
    "1RB 2LA 0LA  1LA 2RA 1RB",
    "1RB 2LA 1LA  0LA 0RB 2RB",
    "1RB 2LB 0LA  1LA 2RB 0RB",
    "1RB 2LB 1LA  1LA 2RB 0RB",
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

    "1RB 0LE  1RC 1RA  1RD 0LA  0LA 1LD  0RB 1LA",  # algebra
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

MACRO_SPINOUT: set[str] = {
    "1RB ...  1RC 0RF  1RD 0LF  1LE 0RC  1LD 0RE  1LC 1RA",
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
    "1RB 1LC  1RC 1RB  1RD 0LE  1LA 1LD  ... 0LA": 3,  # BB(5)
    "1RB 0LC  1RC 1RD  1LA 0RB  0RE ...  1LC 1RA": 2,  # uwe
    "1RB 1RC  0LC 1RD  1LB 1LE  1RD 0RA  1LA 0LE": 3,  # high mark SO

    # 3/3
    "1RB ... 2RB  1LC 0LB 1RA  1RA 2LC 1RC": 2,
    "1RB 2LA 1RA  1RC 2RB 0RC  1LA ... 1LA": 2,

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

PROVER_HALT_TOO_SLOW = {
    "1RB 2LB 4LB 3LA ...  1LA 3RA 3LB 0LB 0RA",
    "1RB ... ... ...  2LC 3RD ... ...  1LA 3RD 1LE 4RD  ... ... 1RF ...  1RF 2LG 2LE 2RH  3RI 2RH 3RJ ...  1LE ... ... 2LC  2LE 2RK 2RH ...  1LE ... ... ...  0RI 1RF 0RJ ...  2RB ... 2RF ...",
}

PROVER_HALT: ProverEst = {
    # 2/5
    "1RB 2LB 4LB 3LA ...  1LA 3RA 3LB 0LB 0RA": (7.3, 19016),
    "1RB 2LA 1RA 2LB 2LA  0LA 2RB 3RB 4RA ...": (1.7, 352),
    "1RB 2LA 4RA 2LB 2LA  0LA 2RB 3RB 4RA ...": (5.2, 105),
    "1RB 2LA 4RA 2LB 2LA  0LA 2RB 3RB 1RA ...": (5.2, 105),
    "1RB 2LA 3RA 2LA 2LB  0LA 2RB 4RB ... 1RA": (5.2, 105),
    "1RB 2LA 1RA 2LB 3RA  0LA 2RB 4RB ... 2RA": (2.4,  88),
    "1RB 2LA 1RA 2LB 2RA  0LA 3RB 3RB 4RA ...": (6.2,  56),
    "1RB 2LA 1RA 2LB 3RA  0LA 2RB 4RB ... 2LA": (1.0,  51),
    "1RB 2LA 4RA 1LB 2LA  0LA 2RB 3RB 2RA ...": (9.3,  30),
    "1RB 2LA 1RA 2RB 2LB  0LA 3RA 4RB ... 1RA": (4.0,  30),
    "1RB 2LA 1RA 2LB 2RA  0LA 2RB 3RB 4RA ...": (3.2,  26),

    # 2/6
    "1RB 2LA 1RA 4LA 5RA 0LB  1LA 3RA 2RB ... 3RB 4LA": "(14 + (2 ** (-1",  # 10 ^^ 70
    "1RB 2LA ... 5LB 5LA 4LB  1LA 4RB 3RB 5LB 1LB 4RA": (1.9, 4933),
    "1RB 1LB 3RA 4LA 2LA 4LB  2LA 2RB 3LB 1LA 5RA ...": (6.9, 4931),
    "1RB 2LB 4RB 1LA 1RB ...  1LA 3RA 5RA 4LB 0RA 4LA": (8.6,  821),
    "1RB 0RB 3LA 5LA ... 4LB  1LA 2RB 3LA 4LB 3RB 3RA": (1.9,   27),

    # 4/3
    "1RB ... 2RC  2LC 2RD 0LC  1RA 2RB 0LB  1LB 0LD 2RC": (1.3, 7036),
    "1RB 0LB 1RD  2RC 2LA 0LA  1LB 0LA 0LA  1RA 0RA ...": (4.2, 6034),
    "1RB 1LD ...  1RC 2LB 2LD  1LC 2RA 0RD  1RC 1LA 0LA": (8.9, 4931),
    "1RB 2LD ...  2LC 2RC 2RB  1LD 0RC 1RC  2LA 2LD 0LB": (2.5, 4561),
    "1RB 1LA 1RD  2LC 0RA 1LB  2LA 0LB 0RD  2RC ... 0LC": (4.0, 3860),

    # 3/4
    "1RB 0LB ... 3LA  0LC 3RB 3RC 1LB  2RB 2LA 3RA 1LC": "(4 + (2 ** (???)))",
    "1RB 1RA 2LB 3LA  2LA 0LB 1LC 1LB  3RB 3RC ... 1LC": (3.7, 6518),
    "1RB 1RA 1LB 1RC  2LA 0LB 3LC ...  1LB 0RC 2RA 2RC": (2.2, 2372),
    "1RB 1LA 3LA 3RC  2LC 2LB 1RB 1RA  2LA 3LC ... 1LB": (1.7, 1301),
    "1RB 3LA 3RC 1RA  2RC 1LA ... 2RB  1LC 1RB 1LB 2RA": (2.1,  628),
    "1RB 0RB 3LC 1RC  0RC ... 2RC 3RC  1LB 2LA 3LA 2RB": (4.6,  434),
    "1RB 1LA 1LB 1RA  0LA 2RB 2LC ...  3RB 2LB 1RC 0RC": (2.4,   26),

    # 6/2
    "1RB 0LD  1RC 0RF  1LC 1LA  0LE ...  1LF 0RB  0RC 0RE": "((-11 + (3 ** ((13",  # 10^^15
    "1RB 0LA  1LC 1LF  0LD 0LC  0LE 0LB  1RE 0RA  ... 1LD": "((38 + (19 * (2 **",  # 10^^5
    "1RB 1RE  1LC 1LF  1RD 0LB  1LE 0RC  1RA 0LD  ... 1LC": "((46 + (49 * (2 **",  # ???
    "1RB ...  0LC 0LD  1LD 1LC  1RE 1LB  1RF 1RD  0LD 0RA": (1.7, 646_456_993),
    "1RB ...  1RC 1RA  1RD 0RB  1LE 0RC  0LF 0LD  0LB 1LA": (2.0, 98641),
    "1RB 1RC  1LC 0RF  1RA 0LD  0LC 0LE  1LD 0RA  1RE ...": (6.0, 39456),
    "1RB 1LE  1RC 1RF  1LD 0RB  1RE 0LC  1LA 0RD  ... 1RC": (3.5, 18267),
    "1RB 0LD  1RC 0RF  1LC 1LA  0LE ...  1LA 0RB  0RC 0RE": (3.1, 10566),
    "1RB 0LE  1LC 0RA  1LD 0RC  1LE 0LF  1LA 1LC  1LE ...": (4.6,  1439),
    "1RB 0RF  0LB 1LC  1LD 0RC  1LE ...  1LF 0LD  1RA 0LE": (2.5,   881),
    "1RB 0LF  0RC 0RD  1LD 1RE  0LE 0LD  0RA 1RC  1LA ...": (1.2,   865),
    "1RB 0LB  0RC 1LB  1RD 0LA  1LE 1LF  1LA 0LD  ... 1LE": (6.4,   462),
    "1RB 0RF  1RC 1RA  1RD 0RC  1LE 0RB  1LB 0LD  ... 0LD": (8.8,   102),
    "1RB 1RF  0RC 0RA  1LD ...  0LE 0LB  1LF 1LD  1RA 1LB": (3.3,    79),
    "1RB 0LC  1LA 1RC  1RA 0LD  1LE 1LC  1RF ...  1RA 1RE": (1.4,    60),
    "1RB 0LB  1LC 0RE  1RE 0LD  1LA 1LA  0RA 0RF  1RE ...": (6.9,    49),
    "1RB 0LC  1LA 1LD  1RD 0RC  0LB 0RE  1RC 1LF  1LE ...": (1.1,    49),
    "1RB 0LC  1LA 1RD  1RA 0LE  1RA 0RB  1LF 1LC  1RD ...": (6.7,    47),
    "1RB 0LC  1LA 1RD  0LB 0LE  1RA 0RB  1LF 1LC  1RD ...": (6.7,    47),
    "1RB 0RC  0LA 0RD  1RD ...  1LE 0LD  1RF 1LB  1RA 1RE": (2.5,    21),
    "1RB 0RC  1RC 0RA  1RD 1RC  1LE 1RF  0LB 1LE  ... 1RB": (3.3,     7),
    "1RB 0LF  1RC 0LE  1RD 0RA  1LE ...  1RC 0LA  1RA 0LB": (1.0,     7),

    # 5/5 block-compiled from 1RB 1RC  1LC 0RF  1RA 0LD  0LC 0LE  1LD 0RA  1RE ...
    "1RB 2LC 3LA 0RD  3LE 2RB 0RB 1LE  1RD 2LA ... ...  1RF 0LA 1RD 2LA  2RF 3RF 1LG ...  1LA 0RF 3LA 0RD  2RB 3RB 1LE 1LH  3LE 3LH ... 1RB": (6.0, 39456),

    # 11/4 backsym-compiled from 2/5 BB
    "1RB ... ... ...  2LC 3RD ... ...  1LA 3RD 1LE 4RD  ... ... 1RF ...  1RF 2LG 2LE 2RH  3RI 2RH 3RJ ...  1LE ... ... 2LC  2LE 2RK 2RH ...  1LE ... ... ...  0RI 1RF 0RJ ...  2RB ... 2RF ...": (7.3, 19016),

    # Green-8
    "1LB ...  0LC 1LC  0LD 0LC  1LE 1RA  0LF 0LE  1LG 1RD  0LH 0LG  1RH 1RF": "((-3 + (7 *",

    # Green-9
    "1RB 1RC  0RD 0RB  ... 1RA  1RE 1LF  0RG 0RE  0RC 1RB  1RH 1LD  0RI 0RH  1LI 1LG": "((-5 + (7 * (3",
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

    # 4/7 block-compiled from 5/2 marks champ
    "1RB 1LC 2LC 2RA  2LD 3RE 1RB ...  0LC 2RA 2LF 3LD  ... 1LF 1RE 3LC  1LC 3LC 3RE 2RG  1RB 1RE 2LC 3RE  0LD 2RA 1RB 3LD": (3.2, 544),

    # 6/2
    "1RB ...  1RC 0RF  1RD 0LF  1LE 0RC  1LD 0RE  1LC 1RA": 2,
    "1RB 0RC  1RC 0RA  1RD 1RC  1LE 1RF  0LB 1LE  1RF 1RB": (3.3, 7),
}

PROVER_QUASIHALT = {
    "1RB 2LC 2RA  1LA 2LB 1RC  1RA 2LC 1RB",
    "1RB 2LA 1RA  1RC 2RB 2RC  1LA 1LB 1LC",
    "1RB 2RC 1LA  2LA 1RB 2LB  2RB 2RA 1LC",
    "1RB 2RC 1LA  2LA 1RB 0RB  2RB 2RA 1LC",
}

RULE_LIMIT = {
    # 2/6
    "1RB 3RB 5RA 1LB 5LA 2LB  2LA 2RA 4RB ... 3LB 2LA": "inapplicable_op",  # 10^^^3
    "1RB 3LA 4LB 0RB 1RA 3LA  2LA 2RA 4LA 1RA 5RB ...": "count_apps",  # 10^^90
}

PROVER_FAILURES = {
    "1RB 1RC 0RC  1RC 0LA 1LB  2LC 2RA 1LB",
    "1RB 0RC 1LA  2LA 0LA 2RB  2RC 2RB 1LC",

    "1RB 0LA  0LC 0RD  0RA 1LB  1RC 1LE  0LE 1LC",

    "1RB 1LB 3RA 4LA 2LA 4LB  2LA 2RB 3LB 1LA 5RA ...",
}

SUSPECTED_RULES = {
    "1RB 2LB 4LB 3LA ...  1LA 3RA 3LB 0LB 0RA",
    "1RB 2LA 1RA 2LB 2LA  0LA 2RB 3RB 4RA ...",
    "1RB 2LA 4RA 2LB 2LA  0LA 2RB 3RB 4RA ...",
    "1RB 2LA 4RA 2LB 2LA  0LA 2RB 3RB 1RA ...",
    "1RB 2LA 3RA 2LA 2LB  0LA 2RB 4RB ... 1RA",
    "1RB 2LA 1RA 2LB 3RA  0LA 2RB 4RB ... 2LA",
    "1RB 2LA 4RA 1LB 2LA  0LA 2RB 3RB 2RA ...",
    "1RB 2LA 1RA 2LB 2RA  0LA 2RB 3RB 4RA ...",

    "1RB ... 2RC  2LC 2RD 0LC  1RA 2RB 0LB  1LB 0LD 2RC",
    "1RB 2LD ...  2LC 2RC 2RB  1LD 0RC 1RC  2LA 2LD 0LB",

    "1RB 1LC  1RC 0RD  0LB 0RC  0RE 1RD  1LE 1LA",
    "1RB 1LC  0RD 0RD  0LB 0RC  0RE 1RD  1LE 1LA",

    "1RB ... ... ...  2LC 3RD ... ...  1LA 3RD 1LE 4RD  ... ... 1RF ...  1RF 2LG 2LE 2RH  3RI 2RH 3RJ ...  1LE ... ... 2LC  2LE 2RK 2RH ...  1LE ... ... ...  0RI 1RF 0RJ ...  2RB ... 2RF ...",
}

REQUIRES_BACKSYM = {
    # 2/5
    "1RB 2LB 4LB 3LA ...  1LA 3RA 3LB 0LB 0RA": 2,

    # 3/4
    "1RB 3LB ... 2RA  2LC 3RB 1LC 2RA  3RB 1LB 3LC 2RC": 1,

    # 6/2
    "1RB 1LD  0RC ...  1RD 0LA  1RE 1RF  1LC 1LE  1RA 0RD": 1,
}

ALGEBRA: dict[str, dict[str, tuple[int, str, str, str]]] = {
    "spinout": {
        "1RB 0LB 1LA  2LC 0LB 2LB  2RC 2RA 0LC": (
            129,
            "0",
            "0",
            "(222 + (11 * (2 ** 41)))",
        ),
        "1RB 0LB 1LA  2LC 0LA 2LB  2RC 2RA 0LC": (
            132,
            "0",
            "0",
            "(222 + (11 * (2 ** 41)))",
        ),
        "1RB 0LB 1LA  2LC 2LB 2LB  2RC 2RA 0LC": (
            141,
            "0",
            "0",
            "(430 + (13 * (2 ** 101)))",
        ),
        "1RB 0RC  1RC 0RA  1RD 1RC  1LE 1RF  0LB 1LE  1RF 1RB": (
            375,
            "(10 ** 8)",
            "(-9 + (2 ** 25))",
            "(478 + (7 * (2 ** 21)))",
        ),
        "1RB ...  1RC 0RF  1RD 0LF  1LE 0RC  1LD 0RE  1LC 1RA": (
            711,
            "1",
            "1",
            "(1231 + (7 * (2 ** 9)))",
        ),
    },

    "halt": {
        "1LB ...  0LC 1LC  0LD 0LC  1LE 1RA  0LF 0LE  1LG 1RD  0LH 0LG  1RH 1RF": (
            152,
            "(10 ** 45)",
            "((-3 + (7 * (3 ** 93))) // 2)",
            "((1041 + (7 * (3 ** 92))) // 2)",
        ),
        "1RB 1LA 3LA 3RC  2LC 2LB 1RB 1RA  2LA 3LC ... 1LB": (
            227,
            "(10 ** 1301)",
            "((-4 + (19 * (2 ** 4320))) // 3)",
            "4817",
        ),
        "1RB ...  1RC 1RA  1RD 0RB  1LE 0RC  0LF 0LD  0LB 1LA": (
            291,
            "(10 ** 98642)",
            "(-5 + (5 * (2 ** (-3 + (5 * (2 ** 16))))))",
            "(718 + (5 * (2 ** 16)))",
        ),
        "1RB 1RA 2LB 3LA  2LA 0LB 1LC 1LB  3RB 3RC ... 1LC": (
            347,
            "(10 ** 6518)",
            "((-5 + (25 * (3 ** 13660))) // 2)",
            "27762",
        ),
        "1RB 0RC  1RC 0RA  1RD 1RC  1LE 1RF  0LB 1LE  ... 1RB": (
            375,
            "(10 ** 8)",
            "(-9 + (2 ** 25))",
            "(478 + (7 * (2 ** 21)))",
        ),
        "1RB 0LD  1RC 0RF  1LC 1LA  0LE ...  1LA 0RB  0RC 0RE": (
            413,
            "(10 ** 10567)",
            "((25 + (3 ** ((29 + (3 ** 11)) // 8))) // 2)",
            "((9861 + ((4 * (3 ** ((21 + (3 ** 11)) // 8))) + ((4 * (3 ** ((21 + (3 ** 11)) // 8))) + ((4 * (3 ** ((13 + (3 ** 11)) // 8))) + ((4 * (3 ** ((13 + (3 ** 11)) // 8))) + ((3 ** 10) * (7 + (4 * (3 ** ((-67 + (3 ** 11)) // 8)))))))))) // 8)",
        ),
        "1RB ...  0LC 0LD  1LD 1LC  1RE 1LB  1RF 1RD  0LD 0RA": (
            522,
            "(10 ** 646456993)",
            "(1 + (2 ** (-1 + (2 ** 31))))",
            "(11559 + ((2 ** 30) * (1 + (2 ** (-33 + (2 ** 31))))))",
        ),
        "1RB 3LA 3RC 1RA  2RC 1LA ... 2RB  1LC 1RB 1LB 2RA": (
            681,
            "(10 ** 629)",
            "((13 + (8 * (7 ** 743))) // 3)",
            "(9481 + (40 * (7 ** 741)))",
        ),
        "1RB 0LB ... 3LA  0LC 3RB 3RC 1LB  2RB 2LA 3RA 1LC": (
            683,
            "(10 ↑↑ 2049)",
            "(???)",
            "(???)",
        ),
        "1RB 2LA ... 5LB 5LA 4LB  1LA 4RB 3RB 5LB 1LB 4RA": (
            731,
            "(10 ** 4933)",
            "(2 ** 16388)",
            "(17326 + (2 ** 16386))",
        ),
        "1RB 0LD  1RC 0RF  1LC 1LA  0LE ...  1LF 0RB  0RC 0RE": (
            980,
            "(10 ↑↑ 16)",
            "((-11 + (3 ** ((13 + (3 ** ((23 + (3 ** ((7 + (3 ** ((21 + (3 ** ((7 + (3 ** ((23 + (3 ** ((7 + (3 ** ((23 + (3 ** ((7 + (3 ** ((21 + (3 ** ((7 + (3 ** ((23 + (3 ** ((7 + (3 ** ((21 + (3 ** 11)) // 8))) // 8))) // 8))) // 8))) // 8))) // 8))) // 8))) // 8))) // 8))) // 8))) // 8))) // 8))) // 8))) // 8))) // 2)",
            "(???)",
        ),
        "1RB 2LB 4RB 1LA 1RB ...  1LA 3RA 5RA 4LB 0RA 4LA": (
            937,
            "(10 ** 822)",
            "((13 + (2 ** 2731)) // 3)",
            "((11963 + (7 * (2 ** 2728))) // 3)",
        ),
        "1RB 0LF  1RC 0LE  1RD 0RA  1LE ...  1RC 0LA  1RA 0LB": (
            992,
            "(10 ** 7)",
            "((2 + (5 * (2 ** 21))) // 3)",
            "((81157 + (35 * (2 ** 20))) // 3)",
        ),
        "1RB 1LD ...  1RC 2LB 2LD  1LC 2RA 0RD  1RC 1LA 0LA": (
            1492,
            "(10 ** 4931)",
            "(3 * (2 ** 16382))",
            "(87714 + (3 * (2 ** 16381)))",
        ),
        "1RB 1RF  0RC 0RA  1LD ...  0LE 0LB  1LF 1LD  1RA 1LB": (
            1630,
            "(10 ** 80)",
            "(142 + (9 * (2 ** 261)))",
            "((~10^17) + ((~10^60) * (2 ** 64)))",
        ),
        "1RB 1RC  1LC 0RF  1RA 0LD  0LC 0LE  1LD 0RA  1RE ...": (
            1698,
            "(10 ** 39456)",
            "(3 * (2 ** 131071))",
            "(1121685 + (9 * (2 ** 131071)))",
        ),
        "1RB 0RF  1RC 1RA  1RD 0RC  1LE 0RB  1LB 0LD  ... 0LD": (
            2335,
            "(10 ** 103)",
            "(1430 + (2 ** 341))",
            "(10863 + (3 * (2 ** 340)))",
        ),
        "1RB 0LB  0RC 1LB  1RD 0LA  1LE 1LF  1LA 0LD  ... 1LE": (
            3147,
            "(10 ** 463)",
            "((-1 + (2 ** 1538)) // 3)",
            "(29554 + (15 * (2 ** 1536)))",
        ),
        "1RB 0LA  1LC 1LF  0LD 0LC  0LE 0LB  1RE 0RA  ... 1LD": (
            3666,
            "(10 ↑↑ 5)",
            "((38 + (19 * (2 ** ((-26 + (7 * (2 ** ((-11 + (7 * (2 ** ((-11 + (19 * (2 ** 69175))) // 9)))) // 9)))) // 9)))) // 9)",
            "(???)",
        ),
        "1RB 2LA 1RA 4LA 5RA 0LB  1LA 3RA 2RB ... 3RB 4LA": (
            5708,
            "(10 ↑↑ 70)",
            "(14 + (2 ** (-1 + (2 ** (2 + (2 ** (1 + (2 ** (-1 + (2 ** (4 + (2 ** (2 ** (-1 + (2 ** (2 ** (2 + (2 ** (2 ** (2 + (2 ** (2 ** (-1 + (2 ** (2 ** (-1 + (2 ** (2 ** (3 + (2 ** (2 ** (3 + (2 ** (2 ** (2 + (2 ** (2 ** (-1 + (2 ** (2 ** (-1 + (2 ** (2 ** (2 + (2 ** (2 ** (4 + (2 ** (2 ** (-1 + (2 ** (2 ** (2 + (2 ** (2 ** (2 + (2 ** (2 ** (-1 + (2 ** (2 ** (3 + (2 ** (2 ** (-1 + (2 ** (4 + (2 ** (2 ** (-1 + (2 ** (2 ** (-1 + (2 ** (2 ** (-1 + (2 ** (2 ** (4 + (2 ** (2 ** (-1 + (2 ** (2 ** (-1 + (2 ** (2 ** (1 + (2 ** (1 + (2 ** (-1 + (2 ** (4 + (2 ** (2 ** (-1 + (2 ** (2 ** (2 + (2 ** (2 ** (2 + (2 ** (2 ** (-1 + (2 ** (2 ** 258))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))",
            "(???)",
        ),
        "1RB 1LE  1RC 1RF  1LD 0RB  1RE 0LC  1LA 0RD  ... 1RC": (
            5723,
            "(10 ** 18267)",
            "((17 + (25 * (2 ** 60680))) // 9)",
            "(((~10^15) + (175 * (2 ** 60674))) // 3)",
        ),
        "1RB 1RE  1LC 1LF  1RD 0LB  1LE 0RC  1RA 0LD  ... 1LC": (
            15200,
            "(10 ↑↑ 5)",
            "((23 + (49 * (2 ** ((-124 + (49 * (2 ** ((-34 + (49 * (2 ** ((62 + (49 * (2 ** 15172))) // 27)))) // 27)))) // 27)))) // 9)",
            "(???)",
        ),
        "1RB 1RC  0RD 0RB  ... 1RA  1RE 1LF  0RG 0RE  0RC 1RB  1RH 1LD  0RI 0RH  1LI 1LG": (
            1793,
            "(10 ↑↑ 30)",
            "((-5 + (7 * (3 ** ((-1 + (7 * (3 ** ((-1 + (7 * (3 ** ((-1 + (7 * (3 ** ((-1 + (7 * (3 ** ((-1 + (7 * (3 ** ((-1 + (7 * (3 ** ((-1 + (7 * (3 ** ((-1 + (7 * (3 ** ((-1 + (7 * (3 ** ((-1 + (7 * (3 ** ((-1 + (7 * (3 ** ((-1 + (7 * (3 ** ((-1 + (7 * (3 ** ((-1 + (7 * (3 ** ((-1 + (7 * (3 ** ((-1 + (7 * (3 ** ((-1 + (7 * (3 ** ((-1 + (7 * (3 ** ((-1 + (7 * (3 ** ((-1 + (7 * (3 ** ((-1 + (7 * (3 ** ((-1 + (7 * (3 ** ((-1 + (7 * (3 ** ((-1 + (7 * (3 ** ((-1 + (7 * (3 ** ((-1 + (7 * (3 ** ((-1 + (7 * (3 ** ((-1 + (7 * (3 ** 31))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 4)",
            "(???)",
        ),
        "1RB 2LC 3LA 0RD  3LE 2RB 0RB 1LE  1RD 2LA ... ...  1RF 0LA 1RD 2LA  2RF 3RF 1LG ...  1LA 0RF 3LA 0RD  2RB 3RB 1LE 1LH  3LE 3LH ... 1RB": (
            1698,
            "(10 ** 39456)",
            "(3 * (2 ** 131071))",
            "(1121685 + (9 * (2 ** 131071)))",
        ),
    },

    "count_apps: (2 ** ((-32 + (2 ** 37)) // 5)) % 100": {
        "1RB 2LA 5LB 0RA 1RA 3LB  1LA 4LA 3LB 3RB 3RB ...": (
            3932,
            "(10 ↑↑ 4)",
            "((322 + ((2 ** 38) * (1 + ((2 ** ((-32 + (2 ** 37)) // 5)) * (1 + (5 * (2 ** ((-155 + ((2 ** 37) * (-1 + (2 ** ((-32 + (2 ** 37)) // 5))))) // 5)))))))) // 5)",
            "((56002 + ((5 * (2 ** ((3 + (2 ** ((153 + (2 ** 37)) // 5))) // 5))) + ((5 * (2 ** ((3 + (2 ** ((153 + (2 ** 37)) // 5))) // 5))) + ((5 * (2 ** ((-2 + (2 ** ((153 + (2 ** 37)) // 5))) // 5))) + ((5 * (2 ** ((-2 + (2 ** ((153 + (2 ** 37)) // 5))) // 5))) + ((23 * (2 ** ((148 + (2 ** 37)) // 5))) + (((2 ** 38) * (1 + (5 * (2 ** ((-42 + (2 ** 37)) // 5))))) + (((2 ** 35) * (49 + (15 * (2 ** ((-37 + (2 ** 37)) // 5))))) + ((2 ** ((158 + (2 ** 37)) // 5)) * (1 + (15 * (2 ** ((-165 + ((2 ** 37) * (-1 + (2 ** ((-32 + (2 ** 37)) // 5))))) // 5))))))))))))) // 5)",
        ),
    },

    "count_apps: (2 ** ((13 + (7 * (2 ** 6373))) // 9)) % 18": {
        "1RB 0RA  0LC 0RE  0LE 1RD  1RC ...  1RA 1LF  0LA 0LB": (
            1698,
            "(10 ↑↑ 4)",
            "((-7 + (7 * (2 ** ((-2 + (7 * (2 ** ((13 + (7 * (2 ** 6373))) // 9)))) // 9)))) // 3)",
            "((561904 + ((21 * (2 ** ((-11 + (7 * (2 ** ((13 + (7 * (2 ** 6373))) // 9)))) // 9))) + ((7 * (2 ** ((4 + (7 * (2 ** 6373))) // 9))) + ((21 * (2 ** ((-5 + (7 * (2 ** 6373))) // 9))) + ((21 * (2 ** ((4 + (7 * (2 ** 6373))) // 9))) + ((21 * (2 ** ((-5 + (7 * (2 ** 6373))) // 9))) + ((2 ** 6371) * (119 + (21 * (2 ** ((-57344 + (7 * (2 ** 6373))) // 9))))))))))) // 9)",
        ),
    },

    "count_apps: (2 ** ((-59 + (61 * (2 ** ((-13 + (61 * (2 ** ((-62 + (61 * (2 ** ((-19 + (61 * (2 ** ((-34 + (61 * (2 ** ((-80 + (61 * (2 ** ((-55 + (61 * (2 ** ((-74 + (61 * (2 ** 105))) // 9)))) // 9)))) // 9)))) // 9)))) // 9)))) // 9)))) // 9)))) // 9)) % 3188646": {
        "1RB 3LA 4LB 0RB 1RA 3LA  2LA 2RA 4LA 1RA 5RB ...": (
            17159,
            "(10 ↑↑ 21)",
            "((122 + (61 * (2 ** ((-74 + (61 * (2 ** ((-53 + (61 * (2 ** ((-77 + (61 * (2 ** ((-43 + (61 * (2 ** ((-22 + (61 * (2 ** ((-34 + (61 * (2 ** ((-80 + (61 * (2 ** ((-55 + (61 * (2 ** ((-74 + (61 * (2 ** ((-59 + (61 * (2 ** ((-77 + (61 * (2 ** ((-59 + (61 * (2 ** ((-13 + (61 * (2 ** ((-62 + (61 * (2 ** ((-19 + (61 * (2 ** ((-34 + (61 * (2 ** ((-80 + (61 * (2 ** ((-55 + (61 * (2 ** ((-74 + (61 * (2 ** 105))) // 9)))) // 9)))) // 9)))) // 9)))) // 9)))) // 9)))) // 9)))) // 9)))) // 9)))) // 9)))) // 9)))) // 9)))) // 9)))) // 9)))) // 9)))) // 9)))) // 9)))) // 9)))) // 9)))) // 3)",
            "(???)",
        ),
    },

    "ops_times: ((-23 + (7 * (3 ** ((-1 + (7 * (3 ** 31))) // 2)))) // 4)": {
        "1LB ...  0LC 1LC  0LD 0LC  1LE 1RA  0LF 0LE  1LG 1RD  0LH 0LG  1LI 1RF  0LJ 0LI  1RJ 1RH": (
            1966,
            "3",
            "3",
            "(???)",
        ),
    },

    "ops_times: (-4 + (2 ** 15))": {
        "1RB 3LB ... 2RA  2LC 3RB 1LC 2RA  3RB 1LB 3LC 2RC": (
            3169,
            "(10 ↑↑ 65533)",
            "(???)",
            "(???)",
        ),
    },

    "inapplicable_op": {
        "1RB 3RB 5RA 1LB 5LA 2LB  2LA 2RA 4RB ... 3LB 2LA": (
            1799,
            "(10 ↑↑ 3)",
            "((52 + (13 * (2 ** ((151 + (13 * (2 ** 803))) // 15)))) // 3)",
            "(???)",
        ),
    },

    "sub_add": {
        "1RB 1LD  0RC 1LE  0RD 1RA  1LD 0LA  0LB 1LE": (
            3314,
            "(10 ↑↑ 6)",
            "(59 + (((((2 ** 50) * (3 + (3 * (2 ** (-48 + (3 * (2 ** 50))))))) + (3 * (2 ** (2 + (3 * (2 ** (2 + (3 * (2 ** 50))))))))) + (3 * (2 ** (2 + (3 * (2 ** (2 + (3 * (2 ** (2 + (3 * (2 ** 50)))))))))))) + (3 * (2 ** (3 * (2 ** (2 + (3 * (2 ** (2 + (3 * (2 ** (2 + (3 * (2 ** 50)))))))))))))))",
            "(???)",
        ),
    },

    "sub_mul": {
        "1RB 0LD  0RC 0RA  1LD 1LE  1RE 1LC  0LE 1LA": (
            2447,
            "(10 ** 7077)",
            "(23540 + ((2 ** 11760) * (-1 + ((2 ** 5879) * (-1 + ((2 ** 2939) * (-1 + ((~10^440) * (2 ** 1469)))))))))",
            "(???)",
        ),
        "1RB 1RC  1LC 1RA  0RC 1RD  1LE 0RB  0LA 0LD": (
            2661,
            "(10 ** 7078)",
            "(23540 + ((2 ** 11760) * (-1 + ((2 ** 5879) * (-1 + ((2 ** 2939) * (-1 + ((~10^441) * (2 ** 1469)))))))))",
            "(???)",
        ),
        "1RB 1RA  1LC 0RF  0LE 0RD  0RE 1LB  1RA 0LC  ... 1RD": (
            12327,
            "9",
            "9",
            "(21880 + ((((2 ** 137) * (1 + ((2 ** (1 + (2 ** 136))) * (1 + (2 ** (1 + (2 ** (137 + (2 ** 136))))))))) + (((2 ** 137) * (1 + ((2 ** (1 + (2 ** 136))) * (1 + (2 ** (1 + (2 ** (137 + (2 ** 136))))))))) + (((2 ** 136) * (1 + ((2 ** (1 + (2 ** 136))) * (1 + (2 ** (1 + (2 ** (137 + (2 ** 136))))))))) + ((((2 ** 137) * (1 + (2 ** (1 + (2 ** 136))))) + ((2 ** 138) * (3 + (5 * (2 ** (-1 + (2 ** 136))))))) + ((2 ** 137) * (1 + ((2 ** (1 + (2 ** 136))) * (1 + (2 ** (1 + (2 ** (137 + (2 ** 136))))))))))))) + ((2 ** 137) * (1 + ((2 ** (1 + (2 ** 136))) * (1 + ((2 ** (1 + (2 ** (137 + (2 ** 136))))) * (1 + (2 ** (1 + (2 ** (138 + ((2 ** 136) * (1 + (2 ** (1 + (2 ** 136)))))))))))))))))",
        ),
    },

    "sub_exp": {
        "1RB 1RF  1RC 1RA  1RD 0RB  1LE 0RC  0LA 0LD  1LE ...": (
            1305,
            "(10 ↑↑ 3)",
            "(6175 + ((2 ** 86) * (3 + (3 * (2 ** (-101 + (15 * (2 ** 86))))))))",
            "(6101 + ((3 * (2 ** (-16 + (15 * (2 ** 86))))) + ((3 * (2 ** (-17 + (15 * (2 ** 86))))) + ((3 * (2 ** (-15 + (3 * (2 ** 88))))) + (((2 ** (-13 + (9 * (2 ** 86)))) * (3 + (3 * (2 ** (-3 + (3 * (2 ** 86))))))) + (((2 ** 86) * (3 + (3 * (2 ** (-100 + (3 * (2 ** 88))))))) + (((2 ** (-10 + (3 * (2 ** 87)))) * (3 + (3 * (2 ** (-4 + (3 * (2 ** 86))))))) + (((2 ** 86) * (3 + (3 * (2 ** (-98 + (9 * (2 ** 86))))))) + (((2 ** (-6 + (3 * (2 ** 86)))) * (3 + (3 * (2 ** (-5 + (3 * (2 ** 86))))))) + (((2 ** 18) * ((~10^21) + (3 * (2 ** (-25 + (3 * (2 ** 86))))))) + ((2 ** 87) * (3 + ((2 ** (-92 + (3 * (2 ** 86)))) * (3 + (3 * (2 ** (-4 + (3 * (2 ** 86)))))))))))))))))))",
        ),
        "1RB 1RE  1LC 0LE  1RD 0LB  1RE 0RA  1LE 1RD": (
            6473,
            "(10 ** 463377092)",
            "(384826393 + (2 ** 1539305379))",
            "((~10^17) + ((2 ** 77) * ((~10^193) + ((2 ** 1368) * (341 + ((2 ** 1466) * (25 + ((2 ** 2930) * (341 + ((2 ** 5870) * (25 + ((2 ** 11738) * (341 + ((2 ** 23486) * (25 + ((2 ** 46970) * (341 + ((2 ** 93950) * (25 + ((2 ** 187898) * (341 + ((2 ** 375806) * (25 + ((2 ** 751610) * (405 + ((2 ** 1503226) * (341 + ((2 ** 3006454) * (25 + ((2 ** 6012906) * (341 + ((2 ** 12025822) * (25 + ((2 ** 24051642) * (341 + ((2 ** 48103294) * (25 + ((2 ** 96206586) * (341 + ((2 ** 192413182) * (25 + ((2 ** 384826362) * (341 + (2 ** 769652734))))))))))))))))))))))))))))))))))))))))))))",
        ),
        "1RB 0RC  1LC 1RA  1RE 0LD  0LC 0LE  0RB 1LD": (
            5940,
            "(10 ** 364014629159)",
            "(((~10^12) + (3 ** 762939453143)) // 2)",
            "((762946627020 + ((3 ** 23) * ((~10^289) + ((3 ** 3103) * (121 + ((3 ** 12501) * (121 + ((3 ** 62501) * (121 + ((3 ** 312501) * (121 + ((3 ** 1562501) * (121 + ((3 ** 7812501) * (121 + ((3 ** 39062501) * (121 + ((3 ** 195312501) * (121 + ((3 ** 976562501) * (121 + ((3 ** 4882812501) * (121 + ((3 ** 24414062501) * (121 + ((3 ** 122070312501) * (121 + (121 * (3 ** 610351562501))))))))))))))))))))))))))))) // 2)",
        ),
        "1RB 0LD  0RC 1LD  1LA 1RE  0LA 0LB  1RC 0RA": (
            6075,
            "(10 ** 546021943734)",
            "(((~10^13) + (3 ** (~10^12))) // 2)",
            "(((~10^12) + ((3 ** 35) * ((~10^432) + ((3 ** 4653) * (121 + ((3 ** 18751) * (121 + ((3 ** 93751) * (121 + ((3 ** 468751) * (121 + ((3 ** 2343751) * (121 + ((3 ** 11718751) * (121 + ((3 ** 58593751) * (121 + ((3 ** 292968751) * (121 + ((3 ** 1464843751) * (121 + ((3 ** 7324218751) * (121 + ((3 ** 36621093751) * (121 + ((3 ** 183105468751) * (121 + (121 * (3 ** 915527343751))))))))))))))))))))))))))))) // 2)",
        ),
        "1RB 0RC  1LB 1RA  1RD 1RB  1LE 0LB  1RA 0LD": (
            8002,
            "(10 ** 463377091)",
            "(384826393 + (3 * (2 ** 1539305378)))",
            "(???)",
        ),
    },

    "sup_mul": {
        "1RB 0RC  1LC 1RA  0RC 1RD  1LE 0RB  1LB 0LD": (
            847,
            "(10 ** 5335)",
            "(29548 + ((2 ** 14772) * (-9849 + ((~10^147) * (2 ** 2460)))))",
            "(124519 + (((2 ** 14771) * (-9849 + ((~10^147) * (2 ** 2460)))) + (((2 ** 14770) * (-9849 + ((~10^147) * (2 ** 2460)))) + ((19923 * (2 ** 65)) + (((2 ** 2458) * ((~10^147) + ((~10^24) * (2 ** 408)))) + (((2 ** 406) * ((~10^24) + (19923 * (2 ** 66)))) + ((2 ** 7) * (-(~10^19) + (39 * (2 ** 66))))))))))",
        ),
        "1RB 2LA 3LA 2LA  3LB 3RA 0RA 0RB": (
            7733,
            "(10 ** 12)",
            "(23 + ((2 ** 5) * (16 + ((2 ** 6) * (16 + ((2 ** 7) * (16 + ((2 ** 8) * (16 + (2 ** 13))))))))))",
            "104",
        ),
        "1RB 0LE  1RC 1RD  0LB ...  1LE 0RF  1RC 1LA  1LE 1RF": (
            4413,
            "(10 ** 18)",
            "(206 + ((2 ** 6) * (1 + ((2 ** 7) * (1 + ((2 ** 8) * (1 + ((2 ** 9) * (9 + (33820809 * (2 ** 4)))))))))))",
            "(23221 + (((2 ** 7) * (1 + ((2 ** 8) * (1 + ((2 ** 9) * (9 + (33820809 * (2 ** 4)))))))) + (((2 ** 8) * (1 + ((2 ** 9) * (9 + (33820809 * (2 ** 4)))))) + (((2 ** 9) * (9 + (33820809 * (2 ** 4)))) + ((33820809 * (2 ** 4)) + ((33820809 * (2 ** 3)) + ((33820809 * (2 ** 3)) + ((((((2 ** 6) * (3 + (2 ** 7))) + ((2 ** 5) * (1 + ((2 ** 6) * (1 + (2 ** 7)))))) + ((2 ** 4) * (1 + ((2 ** 5) * (1 + ((2 ** 6) * (1 + (2 ** 7)))))))) + ((2 ** 3) * (1 + ((2 ** 4) * (1 + ((2 ** 5) * (1 + ((2 ** 6) * (1 + (2 ** 7)))))))))) + ((2 ** 2) * (1 + ((2 ** 3) * (1 + ((2 ** 4) * (1 + ((2 ** 5) * (1 + ((2 ** 6) * (1 + (2 ** 7)))))))))))))))))))",
        ),
    },

    "count-depth": {
        "1RB 0LF  0RC 1RC  1RD 1RB  0RE 1RA  1RF ...  1LA 0LD": (
            2986,
            "(10 ** 1171324)",
            "(1505287 + ((6 ** 752640) * (-1 + ((6 ** 376319) * (-1 + ((6 ** 188159) * (-1 + ((6 ** 94079) * (-1 + ((6 ** 47039) * (-1 + ((6 ** 23519) * (-1 + ((6 ** 11759) * (-1 + ((6 ** 5879) * (-1 + ((6 ** 2939) * (-1 + ((~10^1140) * (6 ** 1469)))))))))))))))))))))",
            "(???)",
        ),
        "1RB 1RE  0LB 1LC  0LD 1RE  1LE 1LB  1RC 0RA": (
            7388,
            "(10 ** 561158)",
            "(2796201 + ((2 ** 1398096) * (8 + ((2 ** 349528) * (1 + ((2 ** 87381) * (1 + ((2 ** 21845) * (1 + ((2 ** 5461) * (1 + ((~10^136) * (2 ** 1365)))))))))))))",
            "(???)",
        ),
        "1RB 0RE  1LC 1RE  1RD 0LB  1RA 0RD  1LC 0RA": (
            4511,
            "(10 ** 428792)",
            "(2848768 + ((2 ** 712191) * (-1 + ((2 ** 356096) * (-1 + ((2 ** 178049) * (-1 + ((2 ** 89025) * (-1 + ((2 ** 44513) * (-1 + ((2 ** 22258) * (-1 + ((2 ** 11130) * (-1 + ((2 ** 5566) * (-1 + ((2 ** 2784) * (-1 + ((~10^425) * (2 ** 1393)))))))))))))))))))))",
            "(???)",
        ),
        "1RB 0LC  1LC 1RA  1LD 0LD  0LE 0LC  1RE 0RB": (
            6250,
            "(10 ↑↑ 7)",
            "(???)",
            "(???)",
        ),
        "1RB 0RC  1LA 1RD  1RF 1LD  0LE 1RA  1LC 0RE  ... 0RA": (
            7160,
            "(10 ** 442169)",
            "((3706935 + ((27 ** 205940) * (-118098 + ((27 ** 68645) * (-258280326 + ((27 ** 22880) * (-564859072962 + ((27 ** 7625) * (-(~10^15) + ((27 ** 2540) * (-(~10^18) + ((27 ** 845) * (-(~10^22) + ((27 ** 280) * (-(~10^25) + ((27 ** 92) * (-(~10^28) + ((27 ** 29) * (-(~10^32) + (13994607 * (27 ** 32))))))))))))))))))))) // 2)",
            "(???)",
        ),
        "1RB 1LC  1LA 1RF  0LD 0LA  0RE 1LF  1LA 1RE  ... 0RE": (
            8012,
            "(10 ** 1466076431)",
            "(481 + ((33820809 * (2 ** 4)) + ((2 ** (435 + (33820809 * (2 ** 4)))) * (8 + ((2 ** (437 + (33820809 * (2 ** 4)))) * (8 + ((2 ** (439 + (33820809 * (2 ** 4)))) * (8 + ((2 ** (441 + (33820809 * (2 ** 4)))) * (8 + ((2 ** (443 + (33820809 * (2 ** 4)))) * (8 + ((2 ** (445 + (33820809 * (2 ** 4)))) * (8 + ((2 ** (447 + (33820809 * (2 ** 4)))) * (8 + ((2 ** (449 + (33820809 * (2 ** 4)))) * (8 + (2 ** (454 + (33820809 * (2 ** 4))))))))))))))))))))))",
            "(???)",
        ),
        "1RB 1LA  1LC 0RA  1LD 0LC  1LE 0LD  0RB 1LD": (
            9527,
            "(10 ** 207779)",
            "(690236 + ((2 ** 345104) * (1 + ((2 ** 172553) * (1 + ((2 ** 86277) * (1 + ((2 ** 43139) * (1 + ((2 ** 21570) * (1 + ((2 ** 10786) * (1 + ((2 ** 5393) * (1 + ((2 ** 2697) * (1 + ((~10^409) * (2 ** 1349)))))))))))))))))))",
            "(???)",
        ),
    },

    "infrul": {
        "1RB 0RD  0RC 0RB  1LC 0LA  0RA 1RE  0LB 1RB": (
            309,
            "3",
            "3",
            "(297 + ((3 ** 28) * (1 + ((3 ** (-27 + (3 ** 27))) * (1 + ((3 ** ((3 ** 27) * (-1 + (3 ** (-27 + (3 ** 27)))))) * (1 + (3 ** ((3 ** (3 ** 27)) * (-1 + (3 ** ((3 ** 27) * (-1 + (3 ** (-27 + (3 ** 27))))))))))))))))",
        ),
        "1RB 1RA  1LC 0RB  1LB 1LD  0RA 0RE  0RB 1RE": (
            386,
            "3",
            "3",
            "(106 + ((3 * (2 ** (-2 + (3 * (2 ** (-2 + (3 * (2 ** (-2 + (3 * (2 ** 46))))))))))) + (((3 * (2 ** (-2 + (3 * (2 ** (-2 + (3 * (2 ** 46)))))))) + (((3 * (2 ** (-2 + (3 * (2 ** 46))))) + ((2 ** 45) * (9 + (3 * (2 ** (-48 + (3 * (2 ** 46)))))))) + (3 * (2 ** (-3 + (3 * (2 ** (-2 + (3 * (2 ** 46)))))))))) + (3 * (2 ** (-3 + (3 * (2 ** (-2 + (3 * (2 ** (-2 + (3 * (2 ** 46))))))))))))))",
        ),
        "1RB 1RA  0LB 1RC  0LD 0LC  1RA 1LE  0LC 1LD": (
            413,
            "(10 ↑↑ 6)",
            "(-7 + (5 * (2 ** (-2 + (5 * (2 ** (-3 + (5 * (2 ** (-3 + (5 * (2 ** (-3 + (5 * (2 ** 17)))))))))))))))",
            "(403 + ((5 * (2 ** (-3 + (5 * (2 ** (-3 + (5 * (2 ** (-3 + (5 * (2 ** 17))))))))))) + (((5 * (2 ** (-3 + (5 * (2 ** (-3 + (5 * (2 ** 17)))))))) + (((5 * (2 ** (-3 + (5 * (2 ** 17))))) + ((2 ** 18) * (5 + (5 * (2 ** (-21 + (5 * (2 ** 17)))))))) + (5 * (2 ** (-3 + (5 * (2 ** (-3 + (5 * (2 ** 17)))))))))) + (5 * (2 ** (-3 + (5 * (2 ** (-3 + (5 * (2 ** (-3 + (5 * (2 ** 17))))))))))))))",
        ),
        "1RB 1RA  1LC 1RE  0RE 1LD  1RE 1LC  1LA 0LE": (
            444,
            "(10 ↑↑ 6)",
            "(-4 + (2 ** (2 + (2 ** (1 + (2 ** (1 + (2 ** (1 + (2 ** 33))))))))))",
            "(136 + ((2 ** 34) * (1 + ((2 ** (-32 + (2 ** 33))) * (1 + ((2 ** ((2 ** 33) * (-1 + (2 ** (-32 + (2 ** 33)))))) * (1 + (2 ** ((2 ** (1 + (2 ** 33))) * (-1 + (2 ** ((2 ** 33) * (-1 + (2 ** (-32 + (2 ** 33))))))))))))))))",
        ),
        "1RB ...  1LC 0RB  1LD 0RD  0RE 1LB  0RC 1RF  0RA 1RE": (
            637,
            "(10 ↑↑ 5)",
            "((17 + (23 * (6 ** ((1 + (69 * (6 ** ((-4 + (69 * (6 ** ((-4 + (69 * (6 ** 13))) // 5)))) // 5)))) // 5)))) // 5)",
            "((429535 + ((69 * (6 ** ((-4 + (69 * (6 ** ((-4 + (69 * (6 ** ((-4 + (69 * (6 ** 13))) // 5)))) // 5)))) // 5))) + (((69 * (6 ** ((-4 + (69 * (6 ** ((-4 + (69 * (6 ** 13))) // 5)))) // 5))) + ((69 * (6 ** ((-4 + (69 * (6 ** ((-4 + (69 * (6 ** 13))) // 5)))) // 5))) + (((69 * (6 ** ((-4 + (69 * (6 ** 13))) // 5))) + ((69 * (6 ** ((-4 + (69 * (6 ** 13))) // 5))) + ((6 ** 14) * (46 + (23 * (6 ** ((-69 + (69 * (6 ** 13))) // 5))))))) + (23 * (6 ** ((1 + (69 * (6 ** ((-4 + (69 * (6 ** 13))) // 5)))) // 5)))))) + (23 * (6 ** ((1 + (69 * (6 ** ((-4 + (69 * (6 ** ((-4 + (69 * (6 ** 13))) // 5)))) // 5)))) // 5)))))) // 5)",
        ),
        "1RB 0RD  0LC 1RE  1RA 1LD  0LC 1LC  0RD 0LE": (
            679,
            "(10 ↑↑ 6)",
            "(-7 + (3 * (2 ** (3 * (2 ** (-1 + (3 * (2 ** (-1 + (3 * (2 ** (-1 + (3 * (2 ** 11))))))))))))))",
            "(596 + ((3 * (2 ** (-1 + (3 * (2 ** (-1 + (3 * (2 ** (-1 + (3 * (2 ** 11))))))))))) + ((3 * (2 ** (-1 + (3 * (2 ** (-1 + (3 * (2 ** (-1 + (3 * (2 ** 11))))))))))) + ((3 * (2 ** (3 * (2 ** (-1 + (3 * (2 ** (-1 + (3 * (2 ** 11)))))))))) + ((3 * (2 ** (-1 + (3 * (2 ** (-1 + (3 * (2 ** (-1 + (3 * (2 ** 11))))))))))) + ((3 * (2 ** (3 * (2 ** (-1 + (3 * (2 ** (-1 + (3 * (2 ** 11)))))))))) + ((3 * (2 ** (-1 + (3 * (2 ** (-1 + (3 * (2 ** 11)))))))) + ((3 * (2 ** (-1 + (3 * (2 ** (-1 + (3 * (2 ** 11)))))))) + ((2 ** 11) * (21 + ((2 ** (-12 + (3 * (2 ** 11)))) * (21 + ((9 * (2 ** ((2 ** 11) * (-3 + (3 * (2 ** (-12 + (3 * (2 ** 11))))))))) + (3 * (2 ** (1 + ((2 ** 11) * (-3 + (3 * (2 ** (-12 + (3 * (2 ** 11)))))))))))))))))))))))",
        ),
        "1RB 1RA  0LC 1RE  0LE 1LD  0LB 1LC  1LA 0LE": (
            495,
            "(10 ↑↑ 6)",
            "((-13 + (7 * (2 ** ((8 + (7 * (2 ** ((8 + (7 * (2 ** ((8 + (7 * (2 ** ((8 + (7 * (2 ** 9560))) // 3)))) // 3)))) // 3)))) // 3)))) // 3)",
            "((64211 + ((7 * (2 ** ((5 + (7 * (2 ** ((8 + (7 * (2 ** ((8 + (7 * (2 ** 9560))) // 3)))) // 3)))) // 3))) + (((7 * (2 ** ((5 + (7 * (2 ** ((8 + (7 * (2 ** 9560))) // 3)))) // 3))) + (((7 * (2 ** ((5 + (7 * (2 ** 9560))) // 3))) + ((2 ** 9560) * (7 + (7 * (2 ** ((-28675 + (7 * (2 ** 9560))) // 3)))))) + (7 * (2 ** ((5 + (7 * (2 ** ((8 + (7 * (2 ** 9560))) // 3)))) // 3))))) + (7 * (2 ** ((5 + (7 * (2 ** ((8 + (7 * (2 ** ((8 + (7 * (2 ** 9560))) // 3)))) // 3)))) // 3)))))) // 3)",
        ),
        "1RB 0LE  1RC 1RA  1RD 0LA  0LA 1LD  0RB 1LA": (
            664,
            "(10 ↑↑ 6)",
            "(-2 + (3 * (2 ** (-5 + (3 * (2 ** (-4 + (3 * (2 ** (-4 + (3 * (2 ** (-4 + (3 * (2 ** 764)))))))))))))))",
            "(1506 + ((3 * (2 ** (-4 + (3 * (2 ** (-4 + (3 * (2 ** (-4 + (3 * (2 ** 764))))))))))) + (((3 * (2 ** (-4 + (3 * (2 ** (-4 + (3 * (2 ** 764)))))))) + (((3 * (2 ** (-4 + (3 * (2 ** 764))))) + ((2 ** 763) * (9 + (3 * (2 ** (-768 + (3 * (2 ** 764)))))))) + (3 * (2 ** (-5 + (3 * (2 ** (-4 + (3 * (2 ** 764)))))))))) + (3 * (2 ** (-5 + (3 * (2 ** (-4 + (3 * (2 ** (-4 + (3 * (2 ** 764))))))))))))))",
        ),
        # "1RB 0RD  0LC 1RE  1RA 1LD  0LC 1LC  0LD 0LE": (
        #     570,
        #     "(10 ↑↑ 6)",
        #     "(-5 + (5 * (2 ** (-3 + (5 * (2 ** (-3 + (5 * (2 ** (-3 + (5 * (2 ** (-3 + (5 * (2 ** 17)))))))))))))))",
        #     "(???)",
        # ),
        "1RB 0LA  1RC 0RF  1RD 0RA  1LE 0RD  1LB 1LE  ... 1RD": (
            695,
            "(10 ↑↑ 6)",
            "(-7 + (3 * (2 ** (-1 + (3 * (2 ** (3 * (2 ** (3 * (2 ** (3 * (2 ** 192))))))))))))",
            "(609 + ((3 * (2 ** (3 * (2 ** (3 * (2 ** (3 * (2 ** 192)))))))) + ((3 * (2 ** (-1 + (3 * (2 ** (3 * (2 ** (3 * (2 ** 192))))))))) + ((3 * (2 ** (-1 + (3 * (2 ** (3 * (2 ** (3 * (2 ** 192))))))))) + ((3 * (2 ** (-2 + (3 * (2 ** (3 * (2 ** (3 * (2 ** 192))))))))) + ((3 * (2 ** (-2 + (3 * (2 ** (3 * (2 ** (3 * (2 ** 192))))))))) + ((9 * (2 ** (-1 + (3 * (2 ** (3 * (2 ** 192))))))) + ((3 * (2 ** (-1 + (3 * (2 ** (3 * (2 ** 192))))))) + ((3 * (2 ** (-2 + (3 * (2 ** (3 * (2 ** 192))))))) + ((3 * (2 ** (-2 + (3 * (2 ** (3 * (2 ** 192))))))) + ((9 * (2 ** (-1 + (3 * (2 ** 192))))) + ((3 * (2 ** (-1 + (3 * (2 ** 192))))) + ((3 * (2 ** (-2 + (3 * (2 ** 192))))) + ((2 ** 191) * (15 + (3 * (2 ** (-193 + (3 * (2 ** 192))))))))))))))))))))",
        ),
        "1RB 0RC  1RC 1RA  1RD 1LA  1LE 0RF  0LA 1LE  ... 0RD": (
            820,
            "(10 ↑↑ 6)",
            "(-4 + (3 ** (3 ** (1 + (3 ** (1 + (3 ** (1 + (3 ** 730)))))))))",
            "(2763 + ((3 ** 729) * (7 + ((3 ** (-729 + (3 ** 730))) * (7 + ((3 ** ((3 ** 730) * (-1 + (3 ** (-729 + (3 ** 730)))))) * (7 + (7 * (3 ** ((3 ** (1 + (3 ** 730))) * (-1 + (3 ** ((3 ** 730) * (-1 + (3 ** (-729 + (3 ** 730)))))))))))))))))",
        ),
        "1RB 2RB 3RB 4RB 5LA 4RA  0LA 1RB 5RA ... ... 1LB": (
            1076,
            "(10 ↑↑ 5)",
            "(1 + (2 ** (2 ** (2 ** (2 ** 16)))))",
            "(905 + ((2 ** 16) * (3 + ((2 ** (-16 + (2 ** 16))) * (3 + ((2 ** ((2 ** 16) * (-1 + (2 ** (-16 + (2 ** 16)))))) * (3 + (2 ** (1 + ((2 ** (2 ** 16)) * (-1 + (2 ** ((2 ** 16) * (-1 + (2 ** (-16 + (2 ** 16)))))))))))))))))",
        ),
        "1RB 0LD  1RC 1LB  1LA 1RE  1LE 1LA  1RC 0RA": (
            1056,
            "(10 ↑↑ 10)",
            "(-1 + (3 * (2 ** (-1 + (2 ** (1 + (2 ** (1 + (2 ** (1 + (2 ** (1 + (2 ** (1 + (2 ** (1 + (2 ** (1 + (2 ** 513)))))))))))))))))))",
            "(???)",
        ),
        "1RB 1LE  1RC 1RE  1RD 1RB  0LB 0LD  1LA 0LA": (
            1050,
            "(10 ↑↑ 11)",
            "(-1 + (7 * (2 ** (-4 + (7 * (2 ** (-3 + (7 * (2 ** (-3 + (7 * (2 ** (-3 + (7 * (2 ** (-3 + (7 * (2 ** (-3 + (7 * (2 ** (-3 + (7 * (2 ** (-3 + (7 * (2 ** (-3 + (7 * (2 ** 53))))))))))))))))))))))))))))))",
            "(???)",
        ),
        "1RB 1LE  1RC 1RE  1RD 1RB  0RE 0LD  1LA 0LA": (
            1206,
            "(10 ↑↑ 11)",
            "(-8 + (3 * (2 ** (3 * (2 ** (-1 + (3 * (2 ** (-1 + (3 * (2 ** (-1 + (3 * (2 ** (-1 + (3 * (2 ** (-1 + (3 * (2 ** (-1 + (3 * (2 ** (-1 + (3 * (2 ** (-1 + (3 * (2 ** 95)))))))))))))))))))))))))))))",
            "(???)",
        ),
        "1RB 1LA  1RC 0RE  1LD 0LA  1LC 0RD  1RC 1RB": (
            1405,
            "(10 ↑↑ 6)",
            "((-8 + (13 * (2 ** ((-5 + (13 * (2 ** ((-5 + (13 * (2 ** ((-5 + (13 * (2 ** ((-5 + (13 * (2 ** 553))) // 3)))) // 3)))) // 3)))) // 3)))) // 3)",
            "(???)",
        ),
        "1RB 2LA 1RA 2RB  2LB 1LA 3RB 1LB": (
            1273,
            "(10 ↑↑ 6)",
            "(2 + (2 ** (-9 + (2 ** (-7 + (2 ** (-7 + (2 ** (-7 + (2 ** 121))))))))))",
            "(407 + ((2 ** 118) * (23 + ((2 ** (-128 + (2 ** 121))) * (23 + ((2 ** ((2 ** 121) * (-1 + (2 ** (-128 + (2 ** 121)))))) * (23 + (23 * (2 ** ((2 ** (-7 + (2 ** 121))) * (-1 + (2 ** ((2 ** 121) * (-1 + (2 ** (-128 + (2 ** 121)))))))))))))))))",
        ),
        "1RB 0LE  0RC 1RB  0RD 1RA  1LD 1LA  1LC 0RB": (
            1548,
            "10",
            "10",
            "(???)",
        ),
        "1RB 0LE  0RC 0LC  0RD 1RA  1LD 1LA  1LC 0RB": (
            1717,
            "10",
            "10",
            "(???)",
        ),
        "1RB 0RD  0RC 0LD  1RD ...  1LE 1LF  1RB 1LD  1RF 0RA": (
            1900,
            "(10 ↑↑ 10)",
            "((5 + (3 ** ((1 + (3 ** ((1 + (3 ** ((1 + (3 ** ((1 + (3 ** ((1 + (3 ** ((1 + (3 ** ((1 + (3 ** ((1 + (3 ** 41)) // 2))) // 2))) // 2))) // 2))) // 2))) // 2))) // 2))) // 2))) // 2)",
            "(???)",
        ),
        "1RB 0RC  0RC 0LD  1RD 1RA  1RE 0LA  1LF ...  1LB 1LF": (
            1914,
            "3",
            "3",
            "(???)",
        ),
        "1RB 0RD  1LC 1LE  1RD 1LB  0LD 1RA  0LA 1LC": (
            2670,
            "(10 ↑↑ 21)",
            "(-1 + (2 ** (-2 + (2 ** (2 ** (2 ** (2 ** (2 ** (2 ** (2 ** (2 ** (2 ** (2 ** (2 ** (2 ** (2 ** (2 ** (2 ** (2 ** (2 ** (2 ** (2 ** 16))))))))))))))))))))))",
            "(???)",
        ),
        "1RB 0RA  1LC 0LB  0LE 0LD  1LB 0LC  1RE 1RA": (
            640,
            "4",
            "4",
            "((40644 + ((5 * (2 ** ((-14 + (5 * (2 ** ((-8 + (5 * (2 ** ((-8 + (5 * (2 ** 24))) // 3)))) // 3)))) // 3))) + (((5 * (2 ** ((-11 + (5 * (2 ** ((-8 + (5 * (2 ** 24))) // 3)))) // 3))) + ((5 * (2 ** ((-14 + (5 * (2 ** ((-8 + (5 * (2 ** 24))) // 3)))) // 3))) + (((5 * (2 ** ((-11 + (5 * (2 ** 24))) // 3))) + ((5 * (2 ** ((-14 + (5 * (2 ** 24))) // 3))) + ((2 ** 21) * (35 + (5 * (2 ** ((-80 + (5 * (2 ** 24))) // 3))))))) + (5 * (2 ** ((-17 + (5 * (2 ** ((-8 + (5 * (2 ** 24))) // 3)))) // 3)))))) + (5 * (2 ** ((-17 + (5 * (2 ** ((-8 + (5 * (2 ** ((-8 + (5 * (2 ** 24))) // 3)))) // 3)))) // 3)))))) // 3)",
        ),
        "1RB 0RA  1RC 1RE  1LD 0LA  1LC 0RD  0RB 1RB": (
            1108,
            "(10 ↑↑ 11)",
            "(4 + (5 * (2 ** (-4 + (5 * (2 ** (-4 + (5 * (2 ** (-4 + (5 * (2 ** (-4 + (5 * (2 ** (-4 + (5 * (2 ** (-4 + (5 * (2 ** (-4 + (5 * (2 ** (-4 + (5 * (2 ** (-4 + (5 * (2 ** 316))))))))))))))))))))))))))))))",
            "(???)",
        ),
        "1RB 0RC  1LA 0RE  1RA 1LD  0LC 0LA  0RD 0RF  0RE ...": (
            1845,
            "15",
            "15",
            "((33555025 + ((2 ** ((-1 + (2 ** ((8 + (2 ** ((8 + (2 ** 88)) // 3))) // 3))) // 3)) + ((3 * (2 ** ((-1 + (2 ** ((8 + (2 ** ((8 + (2 ** 88)) // 3))) // 3))) // 3))) + ((2 ** ((-4 + (2 ** ((8 + (2 ** ((8 + (2 ** 88)) // 3))) // 3))) // 3)) + ((3 * (2 ** ((-4 + (2 ** ((8 + (2 ** ((8 + (2 ** 88)) // 3))) // 3))) // 3))) + ((2 ** 85) * (9 + ((2 ** ((-256 + (2 ** 88)) // 3)) * (9 + ((2 ** (((2 ** 88) * (-1 + (2 ** ((-256 + (2 ** 88)) // 3)))) // 3)) * (9 + (2 ** (((2 ** ((8 + (2 ** 88)) // 3)) * (-1 + (2 ** (((2 ** 88) * (-1 + (2 ** ((-256 + (2 ** 88)) // 3)))) // 3)))) // 3))))))))))))) // 3)",
        ),
        "1RB 0LD  1RC 1RF  0LA 1LF  ... 1LE  0LF 0LA  1RA 1LC": (
            1904,
            "(10 ↑↑ 16)",
            "(1 + (5 * (3 ** (-4 + (5 * (3 ** (-2 + (5 * (3 ** (-2 + (5 * (3 ** (-2 + (5 * (3 ** (-2 + (5 * (3 ** (-2 + (5 * (3 ** (-2 + (5 * (3 ** (-2 + (5 * (3 ** (-2 + (5 * (3 ** (-2 + (5 * (3 ** (-2 + (5 * (3 ** (-2 + (5 * (3 ** (-2 + (5 * (3 ** (-2 + (5 * (3 ** 13)))))))))))))))))))))))))))))))))))))))))))))",
            "(???)",
        ),
        "1RB 1LC  1LA 0LD  1RB 0LA  ... 1LE  1RF 0LB  1RB 0RE": (
            2370,
            "(10 ↑↑ 10)",
            "(-5 + (3 * (2 ** (-2 + (2 ** (2 ** (2 ** (2 ** (2 ** (2 ** (2 ** (2 ** 65536))))))))))))",
            "(???)",
        ),
        "1RB 0LD  1RC 1RF  0LA 1LF  ... 1LE  0LF 0RF  1RA 1LC": (
            3782,
            "(10 ↑↑ 16)",
            "(1 + (5 * (3 ** (-4 + (5 * (3 ** (-2 + (5 * (3 ** (-2 + (5 * (3 ** (-2 + (5 * (3 ** (-2 + (5 * (3 ** (-2 + (5 * (3 ** (-2 + (5 * (3 ** (-2 + (5 * (3 ** (-2 + (5 * (3 ** (-2 + (5 * (3 ** (-2 + (5 * (3 ** (-2 + (5 * (3 ** (-2 + (5 * (3 ** (-2 + (5 * (3 ** 13)))))))))))))))))))))))))))))))))))))))))))))",
            "(???)",
        ),
        "1RB 1LC  0RC 1RE  1LD 1RA  0RE 0LF  1RC 0RD  1LE 1LD": (
            5256,
            "49040",
            "49040",
            "(876451 + (3315 * (2 ** 245)))",
        ),
        "1RB 0RC  1LC 0LC  1LE 0RD  1RA 1LB  0LD 0LF  0LD ...": (
            7219,
            "(10 ↑↑ 25)",
            "((1 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** 16))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)",
            "(???)",
        ),
        "1RB 0RF  1LC 0LC  1LE 0RD  1RA 1LB  0LD 0LE  ... 0RD": (
            7219,
            "(10 ↑↑ 25)",
            "((1 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** 16))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)",
            "(???)",
        ),
        "1RB 0RC  1LC 0LF  1LE 0RD  1RA 1LB  0LD 0LE  ... 0RD": (
            7219,
            "(10 ↑↑ 25)",
            "((1 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** ((-3 + (7 * (5 ** 16))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)))) // 2)",
            "(???)",
        ),
        "1RB 0LA  1RC 1LD  1LB 0RE  1LB 0LB  1LE 0RA": (
            28817,
            "(10 ** 708)",
            "(-15 + (23 * (2 ** 2349)))",
            "(683359 + (713 * (2 ** 2348)))",
        ),
    },

    "depth-limit: ((-17 + (5 * (2 ** (???)))) // 3) % 4": {
        "1RB 1LD  0RC ...  1RD 0LA  1RE 1RF  1LC 1LE  1RA 0RD": (
            5074,
            "(10 ↑↑ 52)",
            "(???)",
            "(???)",
        ),
    },
}

ALGEBRA_PROGS = {
    prog
    for progs in ALGEBRA.values()
    for prog in progs
}

INFRUL: set[str] = {
    "1RB 1LA  0LA 0RB",

    "1RB 0LA ...  1LB 2LA 0RB",
    "1RB 2LA 0RB  1LB 1LA 1RA",
    "1RB ... 0RB  2LB 2LA 0RA",
    "1RB 2LA 0RB  2LB 1LA 0RA",
    "1RB 0LB ...  1LA 2RA 1LA",
    "1RB 0LB ...  2LA ... 0RA",
    "1RB 0LB ...  2LA 2LA 1RA",
    "1RB 1LA 0LA  0LA 2RA ...",
    "1RB 1LA 0LA  1LA 2RA ...",
    "1RB 1LA 0LA  2LA 0RA ...",
    "1RB 1LA 0LA  2LA 2RA ...",
    "1RB 1LB ...  2LA 0RB 1RB",
    "1RB 2LA 0LA  0LA ... 2RA",
    "1RB 2LA 0LA  1LA ... 2RA",
    "1RB 2LA 0LA  2LA ... 2RA",
    "1RB 2LA 0RB  0LA ... 1RA",
    "1RB 2LA 1RB  0LA ... 1RA",
    "1RB 2LA 1RB  1LA ... 1RA",
    "1RB 2LA 1RB  2LA ... 1RA",
    "1RB 2LB ...  1LA 1RB 0RB",
    "1RB 2LB ...  2LA 2LA 1RB",
    "1RB 2LB 1RB  1LA 0RA ...",
    "1RB 2LB 1RB  2LA ... 0RA",
    "1RB 2RB 1LA  0LA 2RA ...",

    "1RB ...  0LC 0RA  0RA 1LB",
    "1RB ...  0LC 0RA  1LA 1LB",
    "1RB ...  1LC 0RA  0RB 0LB",
    "1RB ...  1LC 0RA  1RB 0LB",
    "1RB ...  1LC 0RB  1LB 1LA",
    "1RB ...  1LC 1RA  0RA 0LC",
    "1RB 0LB  1LA 0RC  1RB ...",
    "1RB 0LB  1LC 1RB  ... 1LA",
    "1RB 0LC  0LC 1RA  ... 1LA",
    "1RB 0LC  1LA 0RA  1LA ...",
    "1RB 1LC  0LA 0RB  1LA ...",
    "1RB 1RC  1LC ...  0RA 0LB",
    "1RB ...  0LC 0RB  1LC 1LA",
    "1RB 1LA  0LB 1RC  1LA 0RB",
    "1RB 0LB  0LC 0LA  1RC 1LB",
    "1RB 0LB  1LA 0RC  1LC 1RB",
    "1RB 0LC  1LA 0RA  1RC 1LA",
    "1RB 0RB  0RC 0RA  1LC 1RB",
    "1RB 0RB  1LC 0RA  1LC 1LA",
    "1RB 0RC  1LB 1RA  0RB 0RA",

    "1RB 3LA 1LA ...  1LB 2RB 0LA 0RB",
    "1RB 3LA 1LA 1RA  2LB 2RA ... 0RB",
    "1RB 3LA 1LA 2RA  2LB 3RA ... 0RB",
    "1RB 1LA ... 3LA  2LA 3RB 3LA 0RA",
    "1RB 2LA 3LA 0RB  0LA 2RB 1LB 3RB",
    "1RB 0RB 1RA 3LA  2LB 2RB 3RB 0LA",

    "1RB 0LA  1LC ...  0LD 0LC  1RD 0RA",
    "1RB 1LB  0LB 1RC  1RD 0RC  1LA ...",
    "1RB 1RC  1LC 0LD  1RA 0LB  ... 0LA",
    "1RB 0LD  1LC 1RA  ... 1LA  0RA 1LD",
    "1RB 0LD  0LC 1RA  ... 1LA  0RA 1LD",
    "1RB 1LA  0LA 1LC  0LD 0LC  1RD 1RB",
    "1RB 0RD  1LC 0RA  1LA 1LB  ... 0RC",
    "1RB 1RC  1LC 0RD  1RA 0LB  1RD 0RA",
    "1RB 0LD  1LC 0RA  1LA 1LB  1LD 0LC",
    "1RB 0RC  1LB 0LC  0RD 0LD  1RA 0LA",
    "1RB 1LB  1RC 0RC  0RD 0LD  1LD 0RA",
    "1RB 1LB  1LA 0RC  0LD 1RC  0RB 0LB",
    "1RB 0LD  0LC 1RA  1LA 1LD  1LB 0RA",
    "1RB 0RD  0LC 0RA  1RA 1LD  1LB 0LB",
    "1RB 0RB  0RC 0RD  1LC 1LA  0LC 0LA",
    "1RB 0LD  0RC 1LD  1RD 1RA  1LB 0RA",
    "1RB 0RC  1RC 1LD  1LB 0LD  0RA 0LB",
    "1RB 1LC  1LA 0LC  0LD 1RD  0RB 0RA",
    "1RB 1LC  1LA 1RC  0LD 0RB  1LB 0LA",
    "1RB 1LC  1LA 1RC  0LD 0RB  1RD 0LA",
    "1RB 1LC  1LA 1RC  0RD 0LA  1RA 0RB",
    "1RB 0LD  0RC 0LD  1LC 1LA  0RA 1LD",
    "1RB 0RD  1LC 0LC  0RA 1LB  1RA 0LB",
    "1RB 1LC  1LA 1RC  0LD 0RB  1RA 0LA",
    "1RB 0LD  0LC 1RA  1RC 1LD  1LB 0RA",
    "1RB 0RB  1RC 0LD  1LD 0LB  0RA 1LC",
    "1RB 0LD  0LC 1RA  1RA 1LD  1LB 0RA",
    "1RB 1LC  0LA 1RC  1LA 0RD  0LA 0RB",
    "1RB 1LC  1LA 1RC  0LD 0RB  0RB 0LA",
    "1RB 0LB  1RC 0LD  1LA 0RB  0RC 1LD",
    "1RB 0LC  1RC 0LD  1LA 0RB  0RC 1LD",
    "1RB 1LC  1RC 0LD  1LA 0RB  0RC 1LD",
    "1RB 0LD  1LC 0RA  1RA 0LB  0RB 1LD",
    "1RB 0LC  1RC 0LD  1LD 0RB  0LA 0LA",
    "1RB ...  0LC 0LD  0RC 1LD  1LA 0RB",
    "1RB 0RC  1LB 1RC  0LD 0RB  ... 1LA",

    "1RB 0LA  0LC 0RD  0RA 1LB  1RC 1LE  0LE 1LC",
    "1RB 0LA  0RC 1LA  1RD 1RE  1RE ...  1LB 1LA",
    "1RB 0LA  1RC ...  0RD 1LE  1LC 1RD  0RA 1LA",
    "1RB 0LC  1LA 1RC  1RA 0LD  0RE 1LD  ... 0RA",
    "1RB 1LA  0LA 1RC  1RA 1RD  ... 0RE  0LA 1RC",
    "1RB 1LC  0LA 0RD  1LA 1LE  1RB 1LE  0LC ...",
    "1RB ...  1RC 1LC  1LD 0RE  0RA 0LD  0LB 1RE",
    "1RB 0LA  0RC 1RC  1RD 1LA  1RE 1LB  1LB ...",
    "1RB 0LA  0RC 1RC  1RD 1LA  0RE 1LB  1LC ...",
    "1RB 0RC  1RC 0RA  1LB 0RD  0LE 1RD  1LE 1LB",
    "1RB 0LC  1LC 0LA  0RA 0LD  0RE 1LB  1RE 1RB",
    "1RB 0RE  1RC 1RB  1LD 1LE  0LD 0RA  1LB 1LD",
    "1RB 0RE  0LC 1RD  1LC 1LD  1RE 0RA  0LA 0RB",
    "1RB 0LE  0RC 1LB  1RD 0RD  0LA 1RC  1LE 0LB",
    "1RB 0RE  0LC 1RA  1LC 1LD  1RE 0RE  0LB 0RB",
    "1RB 0RD  1LC 0RC  0LE 1RD  1RA 0RB  1LE 1LA",
    "1RB 1RC  1LC 0RB  0RC 0LD  1LA 0LE  0LA ...",
    "1RB 1LC  0LA 0RD  1LA 1RB  0LE 1RC  1LE 0RB",
    "1RB 1RE  1LC 0LA  1LA 0RD  0LB 1LB  0RE 0LD",
    "1RB 0RE  1RC 1LC  1LD 0LB  0RA 0LC  1RE 0LD",
    "1RB 0RB  0LC 1RD  1LA 1LC  0LE 1RA  1LE 0RD",
    "1RB 0LD  0RC 1RC  1LA 0LE  0RA 1LD  0LA 1RB",
    "1RB 1LD  0RC 1LA  0LD 1RC  0RE 0LA  1LA 0RB",
    "1RB 0RB  1RC 0LD  0LD 1RB  0RA 1LE  1LD 1LA",
    "1RB 0RB  1RC 0LD  1LC 1RB  0RA 1LE  1LD 1LA",
    "1RB 0LD  1RC 1RA  0LA 1RB  0RA 1LE  1LD 0RA",
    "1RB 0LD  1RC 1RA  0LA 0LB  0RA 1LE  1LD 0RA",
    "1RB 0LB  1LC 0RD  1LA 1LB  0RE 1RD  1RC 0LC",
    "1RB 0LB  1LC 1LD  1RD 0LD  1LB 0RE  0RA 1RE",
    "1RB 0LD  1RC 1RA  1LA 0RA  0LE 1LD  1LB 0RB",
    "1RB 1RC  1LC 0RC  1RA 0LD  0LE 1LD  1LA 0RA",
    "1RB 0LD  1RC 0LA  1LD 0RA  1LE 0RA  0RC 1LC",
    "1RB 0LD  1RC 0LA  1LD 0RA  1LE 0RA  0RD 1LC",
    "1RB 0LD  1RC 0LA  1LD 0RA  1LE 1RE  0RC 1LC",
    "1RB 0RD  1LC 0LA  0RA 1LB  1RE 0LB  0LB 1RD",
    "1RB 0RD  1LC 0LA  0RA 1LB  1RE 0LB  1LE 1RD",
    "1RB 0LC  1LC 0RD  0RE 1LA  0LA 1RD  0RB 1LB",
    "1RB ...  0RC 0LA  0LD 0RE  1LB 0LB  1LE 1RC",
    "1RB ...  0LC 0LD  0RC 1LD  0LE 0RB  1RD 1LA",
    "1RB 1LC  0LA ...  0RD 0RE  0LD 1RE  1RA 0LC",
    "1RB 0LE  1RC 0RD  1LC 1RD  0LA 0RC  1LB ...",
    "1RB ...  0RC 0RE  1LD 1RE  0LD 1RA  1LB 0LC",
    "1RB 0LD  1RC 0RD  1RD ...  1LE 0LB  0LE 1RA",
    "1RB 0RD  1LC 0LE  0LC 1RA  0RB 1LA  ... 0LA",
    "1RB 0LC  1RC 0RD  1LB ...  0LA 0RE  1LE 1RD",
    "1RB 0RB  1RC ...  1LD 0LE  1RD 1LE  0RA 0LD",
    "1RB 0RB  0RC 0LD  1LA ...  1LE 1RC  0LE 1RA",
    "1RB 0RD  1RC 0LC  1LA ...  0LB 0RE  1LE 1RD",
    "1RB 0RD  1LC 0LE  0LC 1RA  0RB 1LC  ... 0LA",
    "1RB 0LD  1RC ...  1LA 0LA  0RA 0LE  1RE 1LD",
    "1RB ...  1LC 0RE  1RD 0LD  0LB 0RA  1RE 0LC",
    "1RB 0LD  1RC 0RC  1LD 0LE  0LD 1RB  ... 0LA",
    "1RB ...  1LC 0LC  0RA 0LD  1RD 0LE  0RC 1RA",
    "1RB 0LC  1LC 1RA  ... 0LD  1RD 1LE  0RB 1LE",
    "1RB 0RD  0LC 1RD  1LC 1LA  0LE 0RB  ... 0RC",
    "1RB 0RD  1RC 0RC  1LD 0LE  0LD 1RB  ... 0LA",
    "1RB 1LA  1LC 0RC  0RD 0LB  ... 0LE  1RE 0LA",
    "1RB 0RD  1RC 1LE  0LA ...  1LD 0RE  1LA 0RB",
    "1RB 0RB  1RC 0RD  1LA 0LE  1LD 0RC  0LA ...",
    "1RB 1LB  1LA 0RC  ... 0RD  1RD 0RE  1LE 0LA",
    "1RB ...  1LC 0LC  0RA 0LD  0LE 0RB  1RE 1LD",
    "1RB 0RB  0RC 0RD  1LC 1LA  0LE 1RD  ... 0LA",
    "1RB 0RC  1RC ...  1LD 0LE  0LB 0RA  1LE 1RD",
    "1RB 0LD  1RC 1LE  1LD 0LA  ... 0RB  0LE 1RA",
    "1RB 0LE  1RC ...  1LD 0LA  1RD 1LE  0RA 0LD",
    "1RB 0RB  0RC 0LE  0RD 1LA  1LD 1RC  0LA ...",
    "1RB 0LC  1LA 0RD  0LB 0RA  1RE ...  1RE 0LA",
    "1RB ...  1LC 1RD  0RB 1LA  0LE 0RC  1LE 0RD",
    "1RB 0RC  1LC 0LD  0RB 1RC  1LE ...  1LE 0RA",
    "1RB 0RC  1RC ...  1LC 0RD  0LE 1LB  1LE 0RA",
    "1RB 0LC  1LA ...  0RD 0LD  1RD 1RE  1LE 0LA",
    "1RB 0RB  0LB 1RC  1RD 0RD  1LB 0LE  ... 0LA",
    "1RB ...  0LC 0LE  0RC 1LD  1LA 0RB  0RD 1LE",
    "1RB 0RB  0RC 1RD  1LC 0RD  0LA 0LE  1LA ...",
    "1RB 0RD  1LC 0RE  0LC 1RA  0RB 1LE  0LB ...",
    "1RB 0LD  0LC 1LB  1RC 1LD  0LB 0RE  ... 0RA",
    "1RB 0LB  0LB 1RC  1RD 0RD  1LB 0LE  ... 0LA",
    "1RB 0RE  0RC 0LA  1RD 1LA  1LE ...  1RE 0LC",
    "1RB ...  1LC 0LC  0LE 1LD  0RB 0RA  1RE 0LD",
    "1RB 0LC  1LA 0RD  1LB 0RA  ... 1RE  1RE 0LA",
    "1RB 0RE  1LC 0LC  1LA 0LD  1RD 0LA  0RB ...",
    "1RB 0RB  0RC 1LA  0RD 0LE  1LB ...  1LE 0RA",
    "1RB 0RC  1LA 1LD  0LC 1RD  0RE 0LA  0RB ...",

    "1RB 2LA 0RB  2LB 1LA 0RC  0LB 2RC 1RC",
    "1RB 0RB 2LA  2LB 2RC 0RC  0LB 1RC 1LA",
    "1RB 0LB 2RC  1LA 1LC 2RC  1RC 2LA 0LA",
    "1RB 0RC 2LA  1LA 2LB 0LA  2RC 2RB 1LB",
    "1RB 2RA 0RB  1LC 0LA 2RB  2LC 2LA 1RA",
    "1RB 0RC 1LA  2LA 0LA 2RB  2RC 2RB 1LC",
    "1RB 1RC 0RC  1LC 1RC 2LA  2RC 2LB 0LB",
    "1RB 0RB 2LC  1LC 2RB 0LC  1RA 1LA 0LA",
    "1RB 1LB 0LB  1RC 0RC 2LA  1LA 2RC 0LA",
    "1RB 0RB 1LA  2LC 2RA 0LA  1RA 1LC 0RA",
    "1RB 1LA 0RB  1RC 0RC 1LB  2LA 2RB 0LB",
    "1RB 0RC 0LC  1LB 2RB 0LA  1LA 1RA 2LA",
    "1RB 1LB 2RB  1LC 0LA 0RA  1RC 2LC 0RB",
    "1RB 1LA 0RC  2LB 2RC 1LA  0LA 0RB 0LB",
    "1RB 0RB 2RB  2LC ... 0LA  0LC 1RA 1RB",
    "1RB 0RB 2RB  2LC ... 0LA  0LC 2RA 1RB",
    "1RB 2RC 0RC  1LA ... 0LA  0LC 2LA 2RB",
    "1RB 2RC 0RC  2LA 0LA ...  0LC 2RB 1LA",
    "1RB 2RB 1LA  1LB 2LA 0RC  ... 0LA 0LB",
    "1RB 0RB ...  1LC 2LC 0LA  0LC 2RA 1RB",
    "1RB ... 1LB  1LB 1RC 0RC  2LA 2RC 0LB",
    "1RB 2RC ...  1LB 2RB 0RC  2LB 1RC 0LA",

    "1RB 1RA 0RB 4LA 4LB  2LB 2RB 3RB 2LA 0LA",
    "1RB 2RA 0RB 4LA 3LB  2LB 2RB 3RB 2LA 0LA",
    "1RB 2RB 0RB 2RA 4LA  1LA 3LB 3RB 4RB 0LA",
    "1RB 3RA 1LA 4LA 0RB  2LA 3LB 0LA 2LA 4RB",
    "1RB 3RA 3LA 4LA 0RB  2LA 2LA 0LA 1LB 4RB",
    "1RB 0RB 1RB 2RA 4LA  2LA 3RB 3LB 4RB 0LA",
    "1RB 0RB 3RA 1RB 4LA  2LA 2RB 3LB 4RB 0LA",
}

HALTERS = set(
    HALT
    | HALT_SLOW
    | PROVER_HALT
) | DO_HALT

SPINNERS = set(
    SPINOUT
    | SPINOUT_SLOW
    | SPINOUT_BLANK
    | SPINOUT_BLANK_SLOW
    | PROVER_SPINOUT
) | DO_SPIN_OUT

RECURS = set(
    RECUR_COMPACT
    | RECUR_DIFFUSE
    | RECUR_SLOW
    | RECUR_TOO_SLOW
    | QUASIHALT
) | PROVER_QUASIHALT

UNREASONABLE: set[str] = {
    "1RB 0LA ...  1LB 2LA 0RB",
    "1RB ... 0RB  2LB 2LA 0RA",

    "1RB ...  0LC 0RB  1LC 1LA",

    "1RB 3LA 1LA ...  1LB 2RB 0LA 0RB",
    "1RB 3LA 1LA 1RA  2LB 2RA ... 0RB",
    "1RB 3LA 1LA 2RA  2LB 3RA ... 0RB",

    "1RB 0LA  1LC ...  0LD 0LC  1RD 0RA",
    "1RB 1LB  0LB 1RC  1RD 0RC  1LA ...",
}

OMNIREASONABLE: dict[str, tuple[int, int, int]] = {
    "1RB ...  0LC 0LD  0RC 1LD  1LA 0RB": (6,  8, 10),
    "1RB 0RC  1LB 1RC  0LD 0RB  ... 1LA": (2, 11, 12),

    "1RB 0RB ...  1LC 2LC 0LA  0LC 2RA 1RB": (10,  3, 5),
    "1RB 2RC ...  1LB 2RB 0RC  2LB 1RC 0LA": ( 2, 15, 2),
    "1RB 2RB 1LA  1LB 2LA 0RC  ... 0LA 0LB": ( 6, 12, 2),
    "1RB 0RB 2RB  2LC ... 0LA  0LC 1RA 1RB": ( 7,  6, 8),
    "1RB 0RB 2RB  2LC ... 0LA  0LC 2RA 1RB": ( 7,  6, 8),
    "1RB ... 1LB  1LB 1RC 0RC  2LA 2RC 0LB": ( 5, 12, 7),
    "1RB 2RC 0RC  1LA ... 0LA  0LC 2LA 2RB": ( 7,  8, 9),
    "1RB 2RC 0RC  2LA 0LA ...  0LC 2RB 1LA": ( 7,  8, 9),

    "1RB 0LD  1RC 0RC  1LD 0LE  0LD 1RB  ... 0LA": (10, 10, 11),
    "1RB 0RD  1RC 0RC  1LD 0LE  0LD 1RB  ... 0LA": (10, 10, 11),
    "1RB 0LB  0LB 1RC  1RD 0RD  1LB 0LE  ... 0LA": (11, 10, 11),
    "1RB 0RB  0LB 1RC  1RD 0RD  1LB 0LE  ... 0LA": (11, 10, 11),
    "1RB 0RC  1RC ...  1LD 0LE  0LB 0RA  1LE 1RD": (10, 10, 12),
    "1RB ...  0RC 0LA  0LD 0RE  1LB 0LB  1LE 1RC": (14, 10, 11),
    "1RB 0RB  0RC 1LA  0RD 0LE  1LB ...  1LE 0RA": (10, 12, 13),
    "1RB 0RD  1RC 1LE  0LA ...  1LD 0RE  1LA 0RB": (11, 12, 13),
    "1RB 0RC  1LA 1LD  0LC 1RD  0RE 0LA  0RB ...": (13, 11, 12),
    "1RB ...  0LC 0LD  0RC 1LD  0LE 0RB  1RD 1LA": (12, 11, 13),
    "1RB 1LC  0LA ...  0RD 0RE  0LD 1RE  1RA 0LC": (16, 10, 12),
    "1RB 0LE  1RC 0RD  1LC 1RD  0LA 0RC  1LB ...": (11, 14, 15),
    "1RB 0LD  1RC 0RD  1RD ...  1LE 0LB  0LE 1RA": (18, 11, 13),
    "1RB 0LC  1RC 0RD  1LB ...  0LA 0RE  1LE 1RD": (13, 15, 16),
    "1RB 0RB  0RC 0LD  1LA ...  1LE 1RC  0LE 1RA": (27,  8, 10),
    "1RB 1LA  1LC 0RC  0RD 0LB  ... 0LE  1RE 0LA": ( 6, 19, 20),
    "1RB 0RB  1RC ...  1LD 0LE  1RD 1LE  0RA 0LD": (14, 15, 16),
    "1RB 0RC  1LC 0LD  0RB 1RC  1LE ...  1LE 0RA": ( 3, 20, 22),
    "1RB ...  0LC 0LE  0RC 1LD  1LA 0RB  0RD 1LE": ( 4, 20, 22),
    "1RB 0LD  1RC ...  1LA 0LA  0RA 0LE  1RE 1LD": ( 5, 20, 21),
    "1RB 0RC  1RC ...  1LC 0RD  0LE 1LB  1LE 0RA": ( 3, 21, 22),
    "1RB 0LD  1RC 1LE  1LD 0LA  ... 0RB  0LE 1RA": (29,  7, 10),
    "1RB 0LC  1LA 0RD  0LB 0RA  1RE ...  1RE 0LA": ( 7, 27, 13),
    "1RB 0LC  1LA 0RD  1LB 0RA  ... 1RE  1RE 0LA": ( 7, 27, 13),
    "1RB 0RD  1RC 0LC  1LA ...  0LB 0RE  1LE 1RD": (12, 17, 18),
    "1RB 0LC  1LC 1RA  ... 0LD  1RD 1LE  0RB 1LE": ( 2, 22, 23),
    "1RB 0RE  0RC 0LA  1RD 1LA  1LE ...  1RE 0LC": ( 4, 30, 13),
    "1RB 0RD  1LC 0RE  0LC 1RA  0RB 1LE  0LB ...": (36,  2,  9),
    "1RB ...  1LC 0RE  1RD 0LD  0LB 0RA  1RE 0LC": ( 3, 30, 15),
    "1RB 0LE  1RC ...  1LD 0LA  1RD 1LE  0RA 0LD": ( 7, 20, 21),
    "1RB 1LB  1LA 0RC  ... 0RD  1RD 0RE  1LE 0LA": ( 3, 23, 23),
    "1RB 0RB  0RC 0RD  1LC 1LA  0LE 1RD  ... 0LA": ( 7, 20, 22),
    "1RB 0RD  1LC 0LE  0LC 1RA  0RB 1LC  ... 0LA": (24, 11, 14),
    "1RB ...  1LC 0LC  0RA 0LD  0LE 0RB  1RE 1LD": ( 6, 35,  9),
    "1RB 0RB  0RC 0LE  0RD 1LA  1LD 1RC  0LA ...": ( 6, 36,  9),
    "1RB 0RB  0RC 1RD  1LC 0RD  0LA 0LE  1LA ...": ( 4, 38,  9),
    "1RB 0RB  1RC 0RD  1LA 0LE  1LD 0RC  0LA ...": ( 4, 38,  9),
    "1RB 0RE  1LC 0LC  1LA 0LD  1RD 0LA  0RB ...": ( 4, 38,  9),
    "1RB ...  1LC 0LC  0LE 1LD  0RB 0RA  1RE 0LD": ( 4, 38,  9),
    "1RB 0LD  0LC 1LB  1RC 1LD  0LB 0RE  ... 0RA": ( 2, 43,  7),
    "1RB ...  1LC 1RD  0RB 1LA  0LE 0RC  1LE 0RD": ( 3, 25, 27),
    "1RB 0RD  0LC 1RD  1LC 1LA  0LE 0RB  ... 0RC": ( 2, 27, 28),
    "1RB 0LC  1LA ...  0RD 0LD  1RD 1RE  1LE 0LA": ( 2, 28, 29),
    "1RB ...  1LC 0LC  0RA 0LD  1RD 0LE  0RC 1RA": ( 3, 29, 30),
}

## test program ######################################################

NORMALIZE = {
    "1RB 2LA 1RA 1LA  3LA ... 2RB 2RA": {
        "1RB 3LA 1LA 1RA  2LA ... 3RA 3RB",
        "2RB 2RA 1LA 2LA  3LA 1RB ... 1RA",
        "1LB 2RA 1LA 1RA  3RA ... 2LB 2LA",
        "1LB 3RA 1RA 1LA  2RA ... 3LA 3LB",
        "2LB 2LA 1RA 2RA  3RA 1LB ... 1LA",
    },
    "1RB 1LC  1RD 1RB  0RE 1RE  1LD 1LA  0LF 1LF  0RD 0RC": {
        "1RB 1LE  1RD 1RB  0RD 0RE  1LD 1LA  0RF 1RF  0LC 1LC",
        "1RB 1LD  1RE 1RB  0RE 0RD  0RF 1RF  1LE 1LA  0LC 1LC",
        "1LB 1RC  1LD 1LB  0LE 1LE  1RD 1RA  0RF 1RF  0LD 0LC",
        "1LB 1RE  1LD 1LB  0LD 0LE  1RD 1RA  0LF 1LF  0RC 1RC",
        "1LB 1RD  1LE 1LB  0LE 0LD  0LF 1LF  1RE 1RA  0RC 1RC",
    },

    "1RB 1RC  1LD ...  0RE 0LF  0LF 1LD  1LF ...  1RG 0LF  1RG 1RA": (
        "0RB ...  0LC ...  1LD 1LE  1RF ...  0LG 0RH  0RH 1RF  1RH ...  1LI 0RH  1LI 1LC",
        "1RB 1RC  1LD ...  0RE 0LF  0LF 1LD  1LF ...  1RG 0LF  1RG 1RA  0RA ...  0LH ...",
    ),

    "1RB 1LC 2LC 2RA  2LD 3RE 1RB ...  0LC 2RA 2LF 3LD  ... 1LF 1RE 3LC  1LC 3LC 3RE 2RG  1RB 1RE 2LC 3RE  0LD 2RA 1RB 3LD": (
        "1RB 1LC 2LC 2RA  2LD 3RE 1RB ...  0LC 2RA 2LF 3LD  ... 1LF 1RE 3LC  1LC 3LC 3RE 2RG  1RB 1RE 2LC 3RE  0LD 2RA 1RB 3LD  ... ... ... ...  ... ... ... ...  ... ... ... ...",
    ),

    "1RB 2RC 3RB 1LA 3RC 4LD ... 5RC  6LD 3RB 4RB 2LA 1LA ... ... ...  ... 3RC 1LA 3RB 2RC ... 2LD ...  7RC 1LD 2LD 3LD 4LD 1RB ... 2RB": (
        "1RB 2RC 3RB 1LA 3RC 4LD ... 5RC ...  6LD 3RB 4RB 2LA 1LA ... ... ... ...  ... 3RC 1LA 3RB 2RC ... 2LD ... ...  7RC 1LD 2LD 3LD 4LD 1RB ... 2RB ...",
    ),

    "1RB ... ... ... ...  2LC ... ... ... ...  3RD 3LC ... 1LC ...  ... 1RD 1RB 1LE ...  4RD 1LE ... 1RD 1LC": (
        "1RB ... ... ... ... ... ... ...  2LC ... ... ... ... ... ... ...  3RD 3LC ... 1LC ... ... ... ...  ... 1RD 1RB 1LE ... ... ... ...  4RD 1LE ... 1RD 1LC ... ... ...  ... ... ... ... ... ... ... ...  ... ... ... ... ... ... ... ...  ... ... ... ... ... ... ... ...",
    ),
}

EXPAND = {
    "1RB2RA1LC_2LC1RB2RB_---2LA1LA": "1RB 2RA 1LC  2LC 1RB 2RB  ... 2LA 1LA",
    "1RB3LB1RZ2RA_2LC3RB1LC2RA_3RB1LB3LC2RC": "1RB 3LB ... 2RA  2LC 3RB 1LC 2RA  3RB 1LB 3LC 2RC",
    "1LB1RZ_0LC1LC_0LD0LC_1LE1RA_0LF0LE_1LG1RD_0LH0LG_1LI1RF_0LJ0LI_1RJ1RH": "1LB ...  0LC 1LC  0LD 0LC  1LE 1RA  0LF 0LE  1LG 1RD  0LH 0LG  1LI 1RF  0LJ 0LI  1RJ 1RH",
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
    "1RB 1LB  1LA ...": (
        "BBA.",
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
    "1RB ...  1LB 0RC  1LC 1LA": (
        "B.BCCA",
        1, 1, 0, 1,
        {A: {C}, B: {A, B}, C: {B, C}},
        {A: {B}, B: {B, C}, C: {A, C}},
    ),
    "1RC ...  1LB 0RC  1LB 1LA": (
        "C.BCBA",
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
    "1RB 2LB ...  2LA 2RB 1LB": (
        "BB.ABB",
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
    "1RB 1LB  1LA 0LC  ... 1LD  1RD 0RA": (
        "BBAC.DDA",
        1, 1, 0, 1,
        {A: {B, D}, B: {A}, C: {B}, D: {C, D}},
        {A: {B}, B: {A, C}, C: {D}, D: {A, D}},
    ),
    "1RC 1LB  1LA 0LC  ... 1LD  1RC 0RA": (
        "CBAC.DCA",
        0, 1, 1, 0,
        {A: {B, D}, B: {A}, C: {A, B, D}, D: {C}},
        {A: {B, C}, B: {A, C}, C: {D}, D: {A, C}},
    ),
    "1RB 1LB  1LA 0LB  ... 1LC  1RD 0RA": (
        "BBAB.CDA",
        1, 0, 0, 1,
        {A: {B, D}, B: {A, B}, C: {C}, D: {D}},
        {A: {B}, B: {A, B}, C: {C}, D: {A, D}},
    ),
    "1RC 1LB  1LA 0LC  ... 1LD  1RD 0RD": (
        "CBAC.DDD",
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
    "1RC 0LB  1LD 0LA  1RC 1RD  1LA 0LD": (
        "CBDACDAD",
        0, 1, 0, 1,
        {A: {B, D}, B: {A}, C: {A, C}, D: {B, C, D}},
        {A: {B, C}, B: {A, D}, C: {C, D}, D: {A, D}},
    ),

    # 2 4
    "1RB 2LA 1RA 1RA  1LB 1LA 3RB ...": (
        "BAAABAB.",
        1, 1, 0, 1,
        {A: {A, B}, B: {A, B}},
        {A: {A, B}, B: {A, B}},
    ),
    "1RA 2LA 1RA 1RA  1LB 1LA 3RB ...": (
        "AAAABAB.",
        1, 0, 0, 1,
        {A: {A, B}, B: {B}},
        {A: {A}, B: {A, B}},
    ),

    # 3 3
    "1RB 2LB 1LC  1LA 2RB 1RB  ... 2LA 0LC": (
        "BBCABB.AC",
        1, 1, 0, 0,
        {A: {B, C}, B: {A, B}, C: {A, C}},
        {A: {B, C}, B: {A, B}, C: {A, C}},
    ),
    "1RC 2LB 1LC  1LA 2RB 1RB  ... 2LA 0LC": (
        "CBCABB.AC",
        0, 1, 0, 0,
        {A: {B, C}, B: {A, B}, C: {A, C}},
        {A: {B, C}, B: {A, B}, C: {A, C}},
    ),
    "1RB 1LC ...  1LA 1LC 2RB  1RB 2LC 1RC": (
        "BC.ACBBCC",
        1, 1, 0, 0,
        {A: {B}, B: {A, B, C}, C: {A, B, C}},
        {A: {B, C}, B: {A, B, C}, C: {B, C}},
    ),

    # 5 2
    "1RB 1LC  1RC 1RB  1RD 0LE  1LA 1LD  ... 0LA": (
        "BCCBDEAD.A",
        1, 1, 0, 0,
        {A: {D, E}, B: {A, B}, C: {A, B}, D: {C, D}, E: {C}},
        {A: {B, C}, B: {B, C}, C: {D, E}, D: {A, D}, E: {A}},
    ),
    "1RB 1LC  1RC 1RB  0LE 1RD  1LA 1LD  ... 0LA": (
        "BCCBEDAD.A",
        0, 1, 0, 0,
        {A: {D, E}, B: {A, B}, C: {A, B}, D: {C, D}, E: {C}},
        {A: {B, C}, B: {B, C}, C: {E, D}, D: {A, D}, E: {A}},
    ),
    "1RB 1LC  1RC 1RB  1RD 0LC  1LA 1LD  ... 0LE": (
        "BCCBDCAD.E",
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
