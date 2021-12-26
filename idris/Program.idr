module Program

%default total

public export
Color : Type
Color = Nat

public export
data Shift = L | R

public export
Cast Char Shift where
  cast 'L' = L
  cast 'R' = R
  cast  _  = R

public export
Eq Shift where
  L == L = True
  R == R = True
  _ == _ = False

public export
Show Shift where
  show L = "L"
  show R = "R"

public export
State : Type
State = Nat

public export
halt : State
halt = 0

public export
[CastState]
Cast Char State where
  cast '_' = 0
  cast 'A' = 1
  cast 'B' = 2
  cast 'C' = 3
  cast 'D' = 4
  cast 'E' = 5
  cast 'F' = 6
  cast 'G' = 6
  cast 'H' = 7
  cast  _  = 0

public export
Action : Type
Action = (Color, Shift, State)

public export
Instruction : Type
Instruction = Color -> Action

public export
Program : Type
Program = State -> Instruction
