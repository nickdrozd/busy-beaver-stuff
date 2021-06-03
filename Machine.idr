module Machine

import Program
import public Tape

%default total

public export
interface Tape tape => Machine tape where
  exec : Program -> State -> tape -> (State, tape)
  exec prog state tape =
    let (color, dir, nextState) = prog state $ read tape in
      (nextState, shift dir $ print color tape)

  partial
  run : Nat -> Program -> State -> tape -> (Nat, tape)
  run count prog state tape =
    let (nextState, nextTape) = exec prog state tape in
      case nextState of
        H => (count, nextTape)
        _ => run (S count) prog nextState nextTape

  partial
  runOnBlankTape : Program -> (Nat, tape)
  runOnBlankTape prog = run 1 prog A (the tape blank)


public export
[MicroMachine] Machine MicroTape where
