module Machine

import Program
import public Tape

%default total

public export
interface Tape tape => Machine tape where
  exec : Program -> State -> tape -> Nat -> (State, tape, Nat)
  exec prog state tape steps =
    let
      scan = read tape
      (cx, dir, nextState) = prog state scan
      printed = print cx tape
      skip = if state /= nextState then Nothing else Just (scan, cx)
      (stepped, shifted) = shift dir printed skip
    in
      (nextState, shifted, stepped + steps)

  partial
  run : Program -> State -> tape -> Nat -> (Nat, tape)
  run prog state tape steps =
    let (nextState, nextTape, nextSteps) = exec prog state tape steps in
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
