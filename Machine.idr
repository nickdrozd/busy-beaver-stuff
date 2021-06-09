module Machine

import Program
import public Tape

%default total

public export
interface Tape tape => Machine tape where
  exec : Program -> State -> tape -> Nat -> (State, tape, Nat)
  exec prog state tape count =
    let
      scan = read tape
      (color, dir, nextState) = prog state scan
      printed =
        if color == scan
          then tape
          else print color tape
    in
      (nextState, shift dir printed, S count)

  partial
  run : Program -> State -> tape -> Nat -> (Nat, tape)
  run prog state tape count =
    let (nextState, nextTape, nextCount) = exec prog state tape count in
      case nextState of
        H => (count, nextTape)
        _ => run prog nextState nextTape nextCount

  partial
  runOnBlankTape : Program -> (Nat, tape)
  runOnBlankTape prog = run prog A (the tape blank) 1


public export
[MicroMachine] Machine MicroTape where
