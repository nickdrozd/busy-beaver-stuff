module Machine

import Data.Vect

import Tape
import Program

%default total

public export
interface Tape t => Machine t where
  exec : Program -> State -> t -> (t, State)
  exec prog state tape =
    let (color, shift, nextState) = prog state $ readColor tape in
      (shiftHead (printColor tape color) shift, nextState)

  partial
  runToHalt : Nat -> Program -> State -> t -> (Nat, t)
  runToHalt count prog state tape =
    let (nextTape, nextState) = exec prog state tape in
      case nextState of
        H => (count, nextTape)
        _ => runToHalt (S count) prog nextState nextTape

  partial
  runOnBlankTape : Program -> (Nat, t)
  runOnBlankTape prog = runToHalt 1 prog A (the t blank)


public export
Machine MicroTape where
