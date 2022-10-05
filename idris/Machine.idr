module Machine

import Program
import public Tape

%default total

public export
Cycles : Type
Cycles = Nat

public export
Steps : Type
Steps = Nat

public export
interface
SkipTape tape => Machine tape where
  exec : Program -> State -> tape -> (State, tape, Nat, Bool)
  exec prog state tape =
    let
      (scan, edge) = read tape
      (cx, dir, nextState) = prog state scan
    in
      if state == nextState && checkEdge dir edge
        then (nextState, tape, 0, True)
      else
    let
      shift = if state == nextState then skip else step
      (stepped, shifted) = shift dir tape cx
    in
      (nextState, shifted, stepped, False)
    where
      checkEdge : Shift -> (Maybe Shift) -> Bool
      checkEdge sh (Just dir) = sh == dir
      checkEdge  _          _ = False

  run : Cycles -> Program -> State -> tape -> Steps
        -> IO (Maybe (Steps, Cycles, tape))
  run 0 _ _ _ _ = pure Nothing
  run (S cyclesLeft) prog state tape steps =
    let
      (nextState, nextTape, stepped, recurr) = exec prog state tape
      nextSteps = stepped + steps
    in
      if nextState == halt || blank nextTape || recurr
        then pure $ Just (nextSteps, cyclesLeft, nextTape)
        else run cyclesLeft prog nextState nextTape nextSteps

  runOnBlankTape : Cycles -> Program -> IO (Maybe (Steps, Cycles, tape))
  runOnBlankTape simLim prog = do
    Just (steps, cyclesLeft, tape) <- run simLim prog 1 blankInit 0
      | Nothing => pure Nothing
    pure $ Just (steps, minus simLim cyclesLeft, tape)

  runDouble :
    Cycles -> Program
    -> (State, tape, Steps)
    -> (State, tape, Steps)
    -> IO (Maybe (Steps, Cycles, tape))
  runDouble 0 _ _ _ = pure Nothing
  runDouble (S k) prog (st1, tp1, step) (st2, tp2, slow) =
    let
      (nst1, ntp1, nstep, _) = exec prog st1 tp1
      (nst2, ntp2, nslow, _) =
        if mod step 2 == 0
          then (st2, tp2, Z, False)
          else exec prog st2 tp2
    in
      if nst1 == nst2 && ntp1 == ntp2
        then pure $ Just (slow, 0, tp2) else
      runDouble k prog
        (nst1, ntp1, nstep + step)
        (nst2, ntp2, nslow + slow)

  runDoubleOnBlank :
    Cycles -> Program ->
    IO (Maybe (Steps, Cycles, tape))
  runDoubleOnBlank simLim prog = do
    runDouble simLim prog
      (1, blankInit, 0)
      (1, blankInit, 0)

public export
implementation
[CellMachine] Machine CellTape where

public export
implementation
[BlockMachine] Machine BlockTape where

public export
implementation
[PtrMachine] Machine PtrTape where

public export
implementation
[NumMachine] Machine NumTape where

public export
implementation
[CellVectMachine] Machine CellVectTape where

public export
implementation
[BlockVectMachine] Machine BlockVectTape where
