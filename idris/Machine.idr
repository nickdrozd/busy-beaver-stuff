module Machine

import Program
import public Tape

%default total

public export
interface Tape tape => Machine tape where
  exec : Program -> State -> tape -> (State, tape, Nat, Maybe Integer)
  exec prog state tape =
    let
      scan = read tape
      (cx, dir, nextState) = prog state scan
      (stepped, shifted) = shift dir tape cx $ state == nextState
      marked = case (scan, cx) of
        (Z, S _) => Just $      cast stepped
        (S _, Z) => Just $ -1 * cast stepped
        _        => Nothing
    in
      (nextState, shifted, stepped, marked)

  run : Nat -> Program -> State -> tape -> Nat -> Integer
      -> IO (Nat, tape)
  run 0     _    _     tape steps _     = pure (steps, tape)
  run (S k) prog state tape steps marks =
    let
      (nextState, nextTape, stepped, marked) = exec prog state tape
      nextSteps = stepped + steps
      nextMarks = case marked of
                       Just ms => marks + ms
                       Nothing => marks
    in
      if nextState == H || nextMarks == 0
        then pure (nextSteps, nextTape)
        else run k prog nextState nextTape nextSteps nextMarks

  runOnBlankTape : Program -> IO (Nat, tape)
  runOnBlankTape prog = run limit prog A blank 0 0 where
    limit : Nat
    limit = 5_000_000_000_000

public export
[MicroMachine] Machine MicroTape where

public export
[MacroMachine] Machine MacroTape where
