import Data.Nat  -- comment out this line in Idris 1
import Data.List
import Data.Vect
import Data.Strings

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
Action = (Color, Shift, State)

Instruction : Type
Instruction = Color -> Action

Program : Type
Program = State -> Instruction

Tape : Type
Tape = (len : Nat ** (Vect (S len) Color, Fin (S len)))

----------------------------------------

applyAction : Tape -> Color -> Shift -> Tape
applyAction (len ** (inputTape, pos)) color shift =
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
    (color, shift, nextState) = prog state currColor
  in
  (applyAction (len ** (tape, pos)) color shift, nextState)

MachineResult : Type
MachineResult = (Nat, Tape)

partial
runToHalt : Nat -> Program -> State -> Tape -> MachineResult
runToHalt count prog state tape =
  let (nextTape, nextState) = exec prog state tape in
    case nextState of
      Q0 => (count, nextTape)
      _  => runToHalt (S count) prog nextState nextTape

partial
runOnBlankTape : Program -> MachineResult
runOnBlankTape prog = runToHalt 1 prog Q1 (Z ** ([W], FZ))

----------------------------------------

BB2 : Program

BB2 Q1 W = (B, R, Q2)
BB2 Q1 B = (B, L, Q2)

BB2 Q2 W = (B, L, Q1)
BB2 Q2 B = (B, R, Q0)

BB2 _  c = (c, L, Q0)

-- λΠ> runOnBlankTape BB2
-- (6, MkDPair 3 ([B, B, B, B], FS (FS FZ)))

----------------------------------------

BB3 : Program

BB3 Q1 W = (B, R, Q2)
BB3 Q1 B = (B, R, Q0)

BB3 Q2 W = (B, L, Q2)
BB3 Q2 B = (W, R, Q3)

BB3 Q3 W = (B, L, Q3)
BB3 Q3 B = (B, L, Q1)

BB3 _  c = (c, L, Q0)

-- λΠ> runOnBlankTape BB3
-- (21, MkDPair 4 ([B, B, B, B, B], FS (FS FZ)))

----------------------------------------

BB4 : Program

BB4 Q1 W = (B, R, Q2)
BB4 Q1 B = (B, L, Q2)

BB4 Q2 W = (B, L, Q1)
BB4 Q2 B = (W, L, Q3)

BB4 Q3 W = (B, R, Q0)
BB4 Q3 B = (B, L, Q4)

BB4 Q4 W = (B, R, Q4)
BB4 Q4 B = (W, R, Q1)

BB4 _  c = (c, L, Q0)

-- λΠ> runOnBlankTape BB4
-- (107, MkDPair 13 ([B, W, B, B, B, B, B, B, B, B, B, B, B, B], FS FZ))

----------------------------------------

parseColor : Char -> Maybe Color
parseColor '0' = Just W
parseColor '1' = Just B
parseColor _   = Nothing

parseShift : Char -> Maybe Shift
parseShift 'L' = Just L
parseShift 'R' = Just R
parseShift _   = Nothing

parseState : Char -> Maybe State
parseState 'H' = Just Q0
parseState 'A' = Just Q1
parseState 'B' = Just Q2
parseState 'C' = Just Q3
parseState 'D' = Just Q4
parseState 'E' = Just Q5
parseState 'F' = Just Q6
parseState _   = Nothing

partial
parseAction : String -> Maybe Action
parseAction action = let actionIndex = strIndex action in do
  color <- parseColor $ actionIndex 0
  shift <- parseShift $ actionIndex 1
  state <- parseState $ actionIndex 2
  Just (color, shift, state)

pairUp : List ty -> Maybe (List (Vect 2 ty))
pairUp [ ] = Just []
pairUp [_] = Nothing
pairUp (x1 :: x2 :: xs) = do
  rest <- pairUp xs
  Just $ [x1, x2] :: rest

-- example

rawInput : String
rawInput = "1RB   1LB   1LA   0LC   1RH   1LD   1RD   0RA"

partial
inputWords : List String
inputWords = words rawInput

partial
ex : Maybe (List (Vect 2 Action))
ex = pairUp $ mapMaybe parseAction inputWords

-- ...

Cast State (Fin 4) where
  cast Q1 = FZ
  cast Q2 = FS FZ
  cast Q3 = FS $ FS FZ
  cast Q4 = FS $ FS $ FS FZ
  cast _  = FZ

Cast (Vect 2 Action) Instruction where
  cast [w, b] color =
    case color of
      W => w
      B => b

makeProgram : (Vect 4 (Vect 2 Action)) -> Program
makeProgram actions state = cast $ index (cast state) actions
