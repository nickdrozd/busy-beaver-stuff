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
runToHalt : Nat -> Program -> State -> Tape -> IO MachineResult
runToHalt count prog state tape =
  let (nextTape, nextState) = exec prog state tape in
    case nextState of
      Q0 => pure (count, nextTape)
      _  => do
        putStrLn $ "-> " ++ show count
        runToHalt (S count) prog nextState nextTape

partial
runOnBlankTape : Program -> IO MachineResult
runOnBlankTape prog = runToHalt 1 prog Q1 (Z ** ([W], FZ))

----------------------------------------

Show Color where
  show W = "0"
  show B = "1"

Show Shift where
  show L = "L"
  show R = "R"

Show State where
  show Q0 = "H"
  show Q1 = "A"
  show Q2 = "B"
  show Q3 = "C"
  show Q4 = "D"
  show Q5 = "E"
  show Q6 = "F"

[ShowAction] Show Action where
  show (color, shift, state) =
    show color ++ show shift ++ show state

-- show @{ShowAction} $ bb4 Q1 W

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

-- n is 2; needs to be changed for more colors
BWAction : Type
BWAction = Vect 2 Action

pairUp : List ty -> Maybe (List (Vect 2 ty))
pairUp [ ] = Just []
pairUp [_] = Nothing
pairUp (x1 :: x2 :: xs) = do
  rest <- pairUp xs
  Just $ [x1, x2] :: rest

-- This gets about halfway.
partial
partwayParse : String -> Maybe (List BWAction)
partwayParse input = pairUp $ mapMaybe parseAction $ words input

----------------------------------------

Cast State (Fin 1) where
  cast Q1 = FZ
  cast _  = FZ

Cast State (Fin 2) where
  cast Q2 = FS FZ
  cast x  = weaken $ cast x

Cast State (Fin 3) where
  cast Q3 = FS $ FS FZ
  cast x  = weaken $ cast x

Cast State (Fin 4) where
  cast Q4 = FS $ FS $ FS FZ
  cast x  = weaken $ cast x

Cast State (Fin 5) where
  cast Q5 = FS $ FS $ FS $ FS FZ
  cast x  = weaken $ cast x

Cast State (Fin 6) where
  cast Q6 = FS $ FS $ FS $ FS $ FS FZ
  cast x  = weaken $ cast x

Cast BWAction Instruction where
  cast [w, b] color =
    case color of
      W => w
      B => b

makeProgram : (Cast State $ Fin n) => (Vect n BWAction) -> Program
makeProgram actions state = cast $ index (cast state) actions

----------------------------------------

bb3input : String
bb3input = "1RB   1RH   1LB   0RC   1LC   1LA"

partial
bb3parsed : Maybe (List BWAction)
bb3parsed = partwayParse bb3input

BB3Literal : Vect 3 BWAction
BB3Literal = [
  [(B, (R, Q2)), (B, (R, Q0))],
  [(B, (L, Q2)), (W, (R, Q3))],
  [(B, (L, Q3)), (B, (L, Q1))]]

BB3 : Program
BB3 = makeProgram BB3Literal

-- λΠ> runOnBlankTape BB3
-- (21, MkDPair 4 ([B, B, B, B, B], FS (FS FZ)))

bb3 : Program
bb3 Q1 W = (B, R, Q2)
bb3 Q1 B = (B, R, Q0)
bb3 Q2 W = (B, L, Q2)
bb3 Q2 B = (W, R, Q3)
bb3 Q3 W = (B, L, Q3)
bb3 Q3 B = (B, L, Q1)
bb3 _  c = (c, L, Q0)

----------------------------------------

bb4input : String
bb4input = "1RB   1LB   1LA   0LC   1RH   1LD   1RD   0RA"

partial
bb4parsed : Maybe (List BWAction)
bb4parsed = partwayParse bb4input


BB4Literal : Vect 4 BWAction
BB4Literal = [
  [(B, R, Q2), (B, L, Q2)],
  [(B, L, Q1), (W, L, Q3)],
  [(B, R, Q0), (B, L, Q4)],
  [(B, R, Q4), (W, R, Q1)]]

BB4 : Program
BB4 = makeProgram BB4Literal

-- λΠ> runOnBlankTape BB4
-- (107, MkDPair 13 ([B, W, B, B, B, B, B, B, B, B, B, B, B, B], FS FZ))

bb4 : Program
bb4 Q1 W = (B, R, Q2)
bb4 Q1 B = (B, L, Q2)
bb4 Q2 W = (B, L, Q1)
bb4 Q2 B = (W, L, Q3)
bb4 Q3 W = (B, R, Q0)
bb4 Q3 B = (B, L, Q4)
bb4 Q4 W = (B, R, Q4)
bb4 Q4 B = (W, R, Q1)
bb4 _  c = (c, L, Q0)

----------------------------------------

partial
tm5parse : Maybe $ List BWAction
tm5parse = partwayParse
  "1RB   0LC   1RC   1RD   1LA   0RB   0RE   1RH   1LC   1RA"

tm5 : Program
tm5 = makeProgram [
  [(B, (R, Q2)), (W, (L, Q3))],
  [(B, (R, Q3)), (B, (R, Q4))],
  [(B, (L, Q1)), (W, (R, Q2))],
  [(W, (R, Q5)), (B, (R, Q0))],
  [(B, (L, Q3)), (B, (R, Q1))]]

----------------------------------------

Eq Color where
  (==) B B = True
  (==) W W = True
  (==) _ _ = False

ones : Vect k Color -> Nat
ones xs = let (n ** _) = filter ((/=) W) xs in n

Show Tape where
  show (len ** (tape, pos)) = show (len, ones tape, S $ finToNat pos)

partial
main : IO ()
main = do
  result <- runOnBlankTape tm5
  putStrLn $ "\n*** " ++ show result
  pure ()
