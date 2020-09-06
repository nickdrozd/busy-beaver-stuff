from itertools import product

COLOR = 0, 1, 2
SHIFT = 'L', 'R'
STATE = 'A', 'B'

if __name__ == '__main__':
    actions = product(COLOR, SHIFT, STATE)
    for prog in product(actions, repeat=5):
        print(
            ' '.join(
                ['1RB'] + [
                ''.join([
                    str(instr)
                    for instr in action
                ])
                for action in prog
            ])
        )
