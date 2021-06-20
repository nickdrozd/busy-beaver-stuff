module Program

%default total

public export
Color : Type
Color = Nat

public export
data Shift = L | R

public export
data State = H | -- the halt state
  A | B | C | -- operational states
  D | E | F

public export
Eq State where
  A == A = True
  B == B = True
  C == C = True
  D == D = True
  E == E = True
  F == F = True
  H == H = True
  _ == _ = False

public export
Action : Type
Action = (Color, Shift, State)

public export
Instruction : Type
Instruction = Color -> Action

public export
Program : Type
Program = State -> Instruction
