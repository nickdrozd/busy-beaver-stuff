from typing import Union

State = Union[int, str]
Color = Union[int, str]
Action = tuple[State, Color]

Instr = tuple[int, int, int]
