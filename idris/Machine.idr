module Machine

import Program
import public Tape

%default total

public export
interface
SkipTape tape => Machine tape where
  exec : Program -> State -> tape
         -> (State, tape, Nat, Maybe Integer, Bool)
  exec prog state tape =
    let
      (scan, edge) = read tape
      (cx, dir, nextState) = prog state scan
    in
      if checkEdge dir edge && state == nextState && scan == 0
        then (nextState, tape, 0, Nothing, True)
      else
    let
      shift = if state == nextState then skip else step
      (stepped, shifted) = shift dir tape cx
      marked = case (scan, cx) of
        (Z, S _) => Just $      cast stepped
        (S _, Z) => Just $ -1 * (the Integer $ cast stepped)
        _        => Nothing
    in
      (nextState, shifted, stepped, marked, False)
    where
      checkEdge : Shift -> (Maybe Shift) -> Bool
      checkEdge sh (Just dir) = sh == dir
      checkEdge  _          _ = False

  run : (countdown : Nat) -> Program -> State -> tape -> (Nat, Integer)
        -> IO (Maybe (Nat, tape))
  run 0     _    _     _    _              = pure Nothing
  run (S k) prog state tape (steps, marks) =
    let
      (nextState, nextTape, stepped, marked, recurr) = exec prog state tape
      nextSteps = stepped + steps
      nextMarks = case marked of
                       Just ms => marks + ms
                       Nothing => marks
    in
      if nextState == halt || nextMarks == 0 || recurr
        then pure $ Just (nextSteps, nextTape)
        else run k prog nextState nextTape (nextSteps, nextMarks)

  runOnBlankTape : (limit : Nat) -> Program -> IO (Maybe (Nat, tape))
  runOnBlankTape limit prog = run limit prog 1 blank (0, 0)

  runDouble : Nat -> Program -> (State, State) -> (tape, tape)
              -> (Nat, Nat) -> IO (Maybe (Nat, tape))
  runDouble 0 _ _ _ _ = pure Nothing
  runDouble (S k) prog (st1, st2) (tp1, tp2) (sp1, sp2) =
    let
      (nst1, ntp1, nsp1, _, _) = exec prog  st1  tp1
      (nst2, ntp2, nsp2, _, _) = exec prog  st2  tp2
      (nst3, ntp3, nsp3, _, _) = exec prog nst2 ntp2
    in
      if nst1 == nst3 && ntp1 == ntp3
        then pure $ Just (sp1, tp1) else
      runDouble k prog (nst1, nst3) (ntp1, ntp3) (sp1 + nsp1, sp2 + nsp2 + nsp3)

  runDoubleOnBlank : (limit : Nat) -> Program -> IO (Maybe (Nat, tape))
  runDoubleOnBlank lim prog = do
    runDouble lim prog (1, 1) (blank, blank) (0, 0)

public export
implementation
[MicroMachine] Machine MicroTape where

public export
implementation
[MacroMachine] Machine MacroTape where

public export
implementation
[PtrMachine] Machine PtrTape where

public export
implementation
[NumMachine] Machine NumTape where

public export
implementation
[MicroVectMachine] Machine MicroVectTape where

public export
implementation
[MacroVectMachine] Machine MacroVectTape where
