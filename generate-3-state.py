if __name__ == '__main__':
    actions = tuple(
        color + shift + state
        for color in ('0', '1')
        for shift in ('L', 'R')
        for state in ('A', 'B', 'C', 'H')
    )

    instructions = tuple(
        ' '.join((a1, a2))
        for a1 in actions
        for a2 in actions
    )

    for i1 in instructions:
        for i2 in instructions:
            for i3 in instructions:
                print(' '.join((i1, i2, i3)))
