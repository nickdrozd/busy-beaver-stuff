from tm import Graph

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
        f'  {node} -> {target} [ color=" {COLORS[i]}" ];'
        for node, targets in Graph(prog).arrows.items()
        for i, target in enumerate(targets)
        if target != UNDEFINED and target is not None
    ])

    return f'digraph NAME {{\n{header}\n\n{edges}\n}}'
