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

  shift : Shift -> tape -> Bool -> (Nat, tape)
  shift L =  left
  shift R = right

  left  :          tape -> Bool -> (Nat, tape)
  right :          tape -> Bool -> (Nat, tape)

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

  left (l, c, r) _ =
    let (x, k) = pullNext l in
      (1, (k, x, pushCurr c r))

  right (l, c, r) skip =
    let (s, (k, x, e)) = left (r, c, l) skip in
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

pushCurrBlock : Block -> BlockSpan -> BlockSpan
pushCurrBlock block [] = [block]
pushCurrBlock block@(c, k) ((q, n) :: xs) =
  if c == q
    then (q, k + 1 + n) :: xs
    else block :: (q, n) :: xs

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

  left (l, c, r) _ =
    let (x, k) = pullNextBlock l in
      (1, (k, x, pushCurrBlock (c, 0) r))

  right (l, c, r) skip =
    let (s, (k, x, e)) = left (r, c, l) skip in
      (s, (e, x, k))
