module Machine

import Program
import public Tape

%default total

public export
interface Tape tape => Machine tape where
  exec : Program -> State -> tape -> (State, tape, Nat, Int)
  exec prog state tape =
    let
      scan = read tape
      (cx, dir, nextState) = prog state scan
      (stepped, shifted) = shift dir tape cx $ state == nextState
      marked = case (scan, cx) of
        (Z, S _) =>  1
        (S _, Z) => -1
        _        =>  0
    in
      (nextState, shifted, stepped, marked)

  partial
  run : Program -> State -> tape -> Nat -> Int -> IO (Nat, tape)
  run prog state tape steps marks =
    let
      (nextState, nextTape, stepped, marked) = exec prog state tape
      nextSteps = stepped + steps
      nextMarks = (marked * cast stepped) + marks
    in
      if nextState == H || nextMarks == 0
        then pure (nextSteps, nextTape)
        else run prog nextState nextTape nextSteps nextMarks

  partial
  runOnBlankTape : Program -> IO (Nat, tape)
  runOnBlankTape prog = run prog A blank 0 0

public export
[MicroMachine] Machine MicroTape where

public export
[MacroMachine] Machine MacroTape where
