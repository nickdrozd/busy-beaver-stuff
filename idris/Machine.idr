module Machine

import Program
import public Tape

%default total

public export
interface
Tape tape => Machine tape where
  exec : Program -> State -> tape
         -> (State, tape, Nat, Maybe Integer, Bool)
  exec prog state tape =
    let
      (scan, edge) = read tape
      (cx, dir, nextState) = prog state scan
      shifter = if state == nextState then skipShift else stepShift
      (stepped, shifted) = shifter dir tape cx
      marked = case (scan, cx) of
        (Z, S _) => Just $      cast stepped
        (S _, Z) => Just $ -1 * cast stepped
        _        => Nothing
      recurr = state == nextState && scan == 0 && checkEdge dir edge
    in
      (nextState, shifted, stepped, marked, recurr)
    where
      checkEdge : Shift -> (Maybe Shift) -> Bool
      checkEdge sh (Just dir) = sh == dir
      checkEdge  _          _ = False

  run : (countdown : Nat) -> Program -> State -> tape
        -> (steps : Nat) -> (marks : Integer)
        -> IO (Maybe (Nat, tape))
  run 0     _    _     tape steps _     = pure Nothing
  run (S k) prog state tape steps marks =
    let
      (nextState, nextTape, stepped, marked, recurr) = exec prog state tape
      nextSteps = stepped + steps
      nextMarks = case marked of
                       Just ms => marks + ms
                       Nothing => marks
    in
      if nextState == H || nextMarks == 0 || recurr
        then pure $ Just (nextSteps, nextTape)
        else run k prog nextState nextTape nextSteps nextMarks

  runOnBlankTape : Program -> IO (Maybe (Nat, tape))
  runOnBlankTape prog = run limit prog A blank 0 0 where
    limit : Nat
    limit = 1_000_000_000

public export
implementation
[MicroMachine] Machine MicroTape where

public export
implementation
[MacroMachine] Machine MacroTape where

public export
implementation
[VLenMachine] Machine VLenTape where
