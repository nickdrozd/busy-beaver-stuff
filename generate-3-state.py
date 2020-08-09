COLOR = ('0', '1')
SHIFT = ('L', 'R')
STATE = ('A', 'B', 'C', 'H')

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

        if check_0(i1, '0', 'A'):
            continue

        if check_0(i1, '1', 'A'):
            if check_1(i1, '1', 'A'):
                continue

        for i2 in INSTRUCTIONS:

            if check_0(i1, '1', 'A'):
                if check_1(i1, '1', 'B'):
                    if check_0(i2, '1', 'A'):
                        continue
                    if check_0(i2, '1', 'B'):
                        if check_1(i2, '1', ('A', 'B', 'H')):
                            continue

            if check_0(i1, '0', 'B'):
                if check_0(i2, COLOR, ('A', 'B', 'H')):
                    continue

            for i3 in INSTRUCTIONS:

                if check_0(i1, '0', 'B'):
                    if check_0(i2, '1', 'C'):
                        if check_0(i3, '0', ('A', 'B')):
                            continue

                if i1[0] == i2[0] == i3[0]:
                    continue

                if check_0(i1, '0', 'B'):
                    if check_0(i2, '0', 'C'):
                        if check_0(i3, '1', STATE):
                            continue

                if check_0(i1, '0', 'B'):
                    if check_0(i2, '1', 'C'):
                        if check_0(i3, '0', ('C', 'H')):
                            continue

                print(' '.join((i1, i2, i3)))

# 10616832
# 10485760
# 10223616
#  9699328
#  9666560
#  9633792
#  9437184
#  9420800
#  9414656
#  9381888
#  9349120
