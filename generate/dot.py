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

def make_dot(
        name: str,
        arrows: dict[str, tuple[str, ...]],
) -> str:
    header: str = '\n'.join([
        '  labelloc="t";',
        f'  label="{name}";',
        '  fontname="courier"',
    ]) if len(name) < 50 else ''

    edges: str = '\n'.join([
        f'  {node} -> {target} [ color=" {COLORS[i]}" ];'
        for node, targets in arrows.items()
        for i, target in enumerate(targets)
        if target != UNDEFINED
    ])

    return f'digraph NAME {{\n{header}\n\n{edges}\n}}'
