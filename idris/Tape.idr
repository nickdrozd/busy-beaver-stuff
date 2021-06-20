module Tape

import Data.List

import Program

%default total

public export
interface Tape tape where
  cells : tape -> Nat
  marks : tape -> Nat

  blank : tape

  read  :          tape -> Color
  print : Color -> tape -> tape

  shift : Shift -> tape -> (Nat, tape)
  shift L tape =  left tape
  shift R tape = right tape

  left  :          tape -> (Nat, tape)
  right :          tape -> (Nat, tape)

public export
Tape tape => Show tape where
  show tape = show (cells tape, marks tape)

----------------------------------------

TapeSpan : Type
TapeSpan = List Color

public export
MicroTape : Type
MicroTape = (TapeSpan, Color, TapeSpan)

pullNext : TapeSpan -> (Color, TapeSpan)
pullNext [] = (0, [])
pullNext (x :: xs) = (x, xs)

pushCurr : Color -> TapeSpan -> TapeSpan
pushCurr = (::)

public export
Tape MicroTape where
  cells (l, _, r) = length l + 1 + length r

  marks (l, c, r) = length $ filter (/= 0) $ l ++ [c] ++ r

  blank = ([], 0, [])

  read (_, c, _) = c

  left (l, c, r) =
    let (x, k) = pullNext l in
      (1, (k, x, pushCurr c r))

  right (l, c, r) =
    let (s, (k, x, e)) = left (r, c, l) in
      (s, (e, x, k))

  print cx (l, _, r) = (l, cx, r)

----------------------------------------

Block : Type
Block = (Color, Nat)

BlockSpan : Type
BlockSpan = List Block

public export
MacroTape : Type
MacroTape = (BlockSpan, Color, BlockSpan)

pullNextBlock : BlockSpan -> (Color, BlockSpan)
pullNextBlock [] = (0, [])
pullNextBlock ((c, n) :: xs) =
  (c, case n of
    0   => xs
    S k => (c, k) :: xs)

pushCurrBlock : Color -> BlockSpan -> BlockSpan
pushCurrBlock c [] = [(c, 0)]
pushCurrBlock c ((q, n) :: xs) =
  if c == q
    then (q, S n) :: xs
    else (c, 0) :: (q, n) :: xs

public export
Tape MacroTape where
  cells (l, _, r) =
    S $ foldl (\a, (_, n) => a + 1 + n) 0 $ l ++ r

  marks (l, c, r) =
    (+) (if c == 0 then 0 else 1) $
        foldl (\a, (q, n) =>
                   (+) a $ if q == 0 then 0 else 1 + n)
              0
              (l ++ r)

  blank = ([], 0, [])

  read (_, c, _) = c

  print cx (l, _, r) = (l, cx, r)

  left (l, c, r) =
    let (x, k) = pullNextBlock l in
      (1, (k, x, pushCurrBlock c r))

  right (l, c, r) =
    let (s, (k, x, e)) = left (r, c, l) in
      (s, (e, x, k))
