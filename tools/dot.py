from tm.graph import Graph, show_state as conv

COLORS = (
    'blue',
    'red',
    'forestgreen',
    'purple',
    'goldenrod',
    'black',
    'brown',
    'deeppink',
)

UNDEFINED = '.'

def make_dot(prog: str) -> str:
    header: str = '\n'.join([
        '  labelloc="t";',
        f'  label="{prog}";',
        '  fontname="courier"',
    ]) if len(prog) < 50 else ''

    edges: str = '\n'.join([
        f'  {conv(node)} -> {conv(target)} [ color=" {COLORS[i]}" ];'
        for node, targets in sorted(Graph(prog).arrows.items())
        for i, target in enumerate(targets)
        if target is not None
    ])

    return f'digraph NAME {{\n{header}\n\n{edges}\n}}'
