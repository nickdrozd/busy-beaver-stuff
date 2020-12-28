STATES = (
    'A',
    'B',
    'C',
    'D',
    'E',
    'H',
)


def parse_arrows(prog_string):
    states = iter(
        action[2]
        for action in
        prog_string.split()
    )

    connections = zip(states, states)

    return dict(zip(
        sorted(STATES),
        connections))


def arrows_to_graph_string(arrows):
    return ' '.join([
        connection
        for state in sorted(STATES)
        if state in arrows
        for connection in arrows[state]
    ])
