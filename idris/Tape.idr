module Tape

import Data.Nat
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

  shift : Shift -> tape -> tape
  shift L tape =  left tape
  shift R tape = right tape

  left  :          tape -> tape
  right :          tape -> tape

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
      (k, x, pushCurr c r)

  right (l, c, r) =
    let (k, x, e) = left (r, c, l) in
      (e, x, k)

  print cx (l, _, r) = (l, cx, r)
