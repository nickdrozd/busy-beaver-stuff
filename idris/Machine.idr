module Machine

import Program
import public Tape

%default total

public export
interface Tape tape => Machine tape where
  exec : Program -> State -> tape -> (State, tape, Nat)
  exec prog state tape =
    let
      scan = read tape
      (cx, dir, nextState) = prog state scan
      skip = state == nextState
      (stepped, shifted) = shift dir tape cx skip
    in
      (nextState, shifted, stepped)

  partial
  run : Program -> State -> tape -> Nat -> (Nat, tape)
  run prog state tape steps =
    let
      (nextState, nextTape, stepped) = exec prog state tape
      nextSteps = stepped + steps
    in
      case nextState of
        H => (nextSteps, nextTape)
        _ => run prog nextState nextTape nextSteps

  partial
  runOnBlankTape : Program -> (Nat, tape)
  runOnBlankTape prog = run prog A (the tape blank) 0

public export
[MicroMachine] Machine MicroTape where

public export
[MacroMachine] Machine MacroTape where
