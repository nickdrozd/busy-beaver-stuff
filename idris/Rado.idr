-- Turing machine, along with some Busy Beaver programs

-- works in Idris 2

-- comment out these lines in Idris 1
-------------------
import Data.Nat
import Data.Strings
-------------------
import Data.List
import Data.Vect

%default total

----------------------------------------

Color : Type
Color = Nat

blank : Color
blank = 0

data Shift = L | R

data State = H | -- the halt state
  A | B | C | -- operational states
  D | E | F

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
      H => pure (count, nextTape)
      _ => do
        putStrLn $ "-> " ++ show count
        runToHalt (S count) prog nextState nextTape

partial
runOnBlankTape : Program -> IO MachineResult
runOnBlankTape prog = runToHalt 1 prog A (Z ** ([blank], FZ))

----------------------------------------

Show Shift where
  show L = "L"
  show R = "R"

Show State where
  show H = "H"
  show A = "A"
  show B = "B"
  show C = "C"
  show D = "D"
  show E = "E"
  show F = "F"

[ShowAction] Show Action where
  show (color, shift, state) =
    show color ++ show shift ++ show state

-- show @{ShowAction} $ bb4 A 0

----------------------------------------

BB2 : Program

BB2 A 0 = (1, R, B)
BB2 A 1 = (1, L, B)

BB2 B 0 = (1, L, A)
BB2 B 1 = (1, R, H)

BB2 _  c = (c, L, H)

-- λΠ> runOnBlankTape BB2
-- (6, MkDPair 3 ([1, 1, 1, 1], FS (FS FZ)))

----------------------------------------

-- For parse format, see http://www.logique.jussieu.fr/~michel/ha.html

parseColor : Char -> Maybe Color
parseColor '0' = Just 0
parseColor '1' = Just 1
parseColor _   = Nothing

parseShift : Char -> Maybe Shift
parseShift 'L' = Just L
parseShift 'R' = Just R
parseShift _   = Nothing

parseState : Char -> Maybe State
parseState 'H' = Just H
parseState 'A' = Just A
parseState 'B' = Just B
parseState 'C' = Just C
parseState 'D' = Just D
parseState 'E' = Just E
parseState 'F' = Just F
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
  cast A = FZ
  cast _  = FZ

Cast State (Fin 2) where
  cast B = FS FZ
  cast x  = weaken $ cast x

Cast State (Fin 3) where
  cast C = FS $ FS FZ
  cast x  = weaken $ cast x

Cast State (Fin 4) where
  cast D = FS $ FS $ FS FZ
  cast x  = weaken $ cast x

Cast State (Fin 5) where
  cast E = FS $ FS $ FS $ FS FZ
  cast x  = weaken $ cast x

Cast State (Fin 6) where
  cast F = FS $ FS $ FS $ FS $ FS FZ
  cast x  = weaken $ cast x

Cast BWAction Instruction where
  cast [w, b] color =
    case color of
      0 => w
      1 => b
      _  => b

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
  [(1, (R, B)), (1, (R, H))],
  [(1, (L, B)), (0, (R, C))],
  [(1, (L, C)), (1, (L, A))]]

BB3 : Program
BB3 = makeProgram BB3Literal

-- λΠ> runOnBlankTape BB3
-- (21, MkDPair 4 ([1, 1, 1, 1, 1], FS (FS FZ)))

bb3 : Program
bb3 A 0 = (1, R, B)
bb3 A 1 = (1, R, H)
bb3 B 0 = (1, L, B)
bb3 B 1 = (0, R, C)
bb3 C 0 = (1, L, C)
bb3 C 1 = (1, L, A)
bb3 _ c = (c, L, H)

----------------------------------------

bb4input : String
bb4input = "1RB   1LB   1LA   0LC   1RH   1LD   1RD   0RA"

partial
bb4parsed : Maybe (List BWAction)
bb4parsed = partwayParse bb4input


BB4Literal : Vect 4 BWAction
BB4Literal = [
  [(1, R, B), (1, L, B)],
  [(1, L, A), (0, L, C)],
  [(1, R, H), (1, L, D)],
  [(1, R, D), (0, R, A)]]

BB4 : Program
BB4 = makeProgram BB4Literal

-- λΠ> runOnBlankTape BB4
-- (107, MkDPair 13 ([1, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1], FS FZ))

bb4 : Program
bb4 A 0 = (1, R, B)
bb4 A 1 = (1, L, B)
bb4 B 0 = (1, L, A)
bb4 B 1 = (0, L, C)
bb4 C 0 = (1, R, H)
bb4 C 1 = (1, L, D)
bb4 D 0 = (1, R, D)
bb4 D 1 = (0, R, A)
bb4 _ c = (c, L, H)

----------------------------------------

partial
tm5parse : Maybe $ List BWAction
tm5parse = partwayParse
  "1RB   0LC   1RC   1RD   1LA   0RB   0RE   1RH   1LC   1RA"

tm5 : Program
tm5 = makeProgram [
  [(1, (R, B)), (0, (L, C))],
  [(1, (R, C)), (1, (R, D))],
  [(1, (L, A)), (0, (R, B))],
  [(0, (R, E)), (1, (R, H))],
  [(1, (L, C)), (1, (R, A))]]

-- (134467, (667, 501))

----------------------------------------

partial
bb5parse : Maybe $ List BWAction
bb5parse = partwayParse
  "1RB   1LC   1RC   1RB   1RD   0LE   1LA   1LD   1RH   0LA"

bb5 : Program
bb5 = makeProgram [
  [(1, (R, B)), (1, (L, C))],
  [(1, (R, C)), (1, (R, B))],
  [(1, (R, D)), (0, (L, E))],
  [(1, (L, A)), (1, (L, D))],
  [(1, (R, H)), (0, (L, A))]]

-- 791146...

----------------------------------------

ones : Vect k Color -> Nat
ones xs = let (n ** _) = filter ((/=) blank) xs in n

Show Tape where
  show (_ ** (tape, _)) = show (length tape, ones tape)

partial
main : IO ()
main = do
  result <- runOnBlankTape tm5
  putStrLn $ "\n*** " ++ show result
  pure ()
