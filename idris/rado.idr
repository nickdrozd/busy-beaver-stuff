import Data.Nat  -- comment out this line in Idris 1
import Data.Vect

%default total

----------------------------------------

data Color = W | B

blank : Color
blank = W

data Shift = L | R

data State = Q0 | -- the halt state
   Q1 | Q2 | Q3 | -- operational states
   Q4 | Q5 | Q6

Action : Type
Action = (Color, Shift)

Program : Type
Program = State -> Color -> (Action, State)

Tape : Type
Tape = (len : Nat ** (Vect (S len) Color, Fin (S len)))

----------------------------------------

applyAction : Tape -> Action -> Tape
applyAction (len ** (inputTape, pos)) (color, shift) =
  let tape = replaceAt pos color inputTape in
    case pos of

      -- on the leftmost square
      FZ => case shift of
         -- moving further left; add a blank to the left and go there
         L => (S len ** ([blank] ++ tape, FZ))
         R => case len of
           -- there is just one square; leftmost is also rightmost,
           -- so add a blank to the right and go there
           Z   => (S len ** (tape ++ [blank], FS FZ))
           -- move to one right of leftmost
           S _ => (len ** (tape, FS FZ))

      -- some square to the left of the leftmost
      FS p => case shift of
           -- somewhere inside the bounds of the tape; just go left
           L => (len ** (tape, weaken p))
           -- are we at the rightmost square?
           R => case strengthen pos of
             -- no; just move right
             Right bound => (len ** (tape, FS bound))
             -- yes; add a blank square to the right and go there
             Left _ =>
               let prf = sym $ plusCommutative len 1 in
                 (S len ** (rewrite prf in tape ++ [blank], FS pos))

exec : Program -> State -> Tape -> (Tape, State)
exec prog state (len ** (tape, pos)) =
  let
    currColor = index pos tape
    (action, nextState) = prog state currColor
  in
  (applyAction (len ** (tape, pos)) action, nextState)

partial
runToHalt : Nat -> Program -> State -> Tape -> (Nat, Tape)
runToHalt count prog state tape =
  let (nextTape, nextState) = exec prog state tape in
    case nextState of
      Q0 => (count, nextTape)
      _  => runToHalt (S count) prog nextState nextTape

partial
runOnBlankTape : Program -> (Nat, Tape)
runOnBlankTape prog = runToHalt 1 prog Q1 (Z ** ([W], FZ))
