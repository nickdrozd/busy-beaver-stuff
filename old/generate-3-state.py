COLOR = ('0', '1')
SHIFT = ('L', 'R')
STATE = ('A', 'B', 'C')

ACTIONS = tuple(
    color + shift + state
    for color in COLOR
    for shift in SHIFT
    for state in STATE
)

INSTRUCTIONS = tuple(
    ' '.join((a1, a2))
    for a1 in ACTIONS
    for a2 in ACTIONS
)

def check_0(instr, color, state):
    return (
        (instr[0] == color or instr[0] in color)
        and (instr[2] == state or instr[2] in state))

def check_1(instr, color, state):
    return (
        (instr[4] == color or instr[4] in color)
        and (instr[6] == state or instr[6] in state))


if __name__ == '__main__':

    for i1 in INSTRUCTIONS:
        for i2 in INSTRUCTIONS:
            for i3 in INSTRUCTIONS:
                print(' '.join((i1, i2, i3)))
