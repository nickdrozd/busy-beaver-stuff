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

public export
Tape MicroTape where
  cells (l, _, r) = length l + 1 + length r

  marks (l, c, r) =
      length (filter (/= 0) l)
    + (if c == 0 then 0 else 1)
    + length (filter (/= 0) r)

  blank = ([], 0, [])

  read (_, c, _) = c

  left  (    [], c, rs) = ([], 0, c :: rs)
  left  (h :: t, c, rs) = ( t, h, c :: rs)

  right (ls, c, []    ) = (c :: ls, 0, [])
  right (ls, c, h :: t) = (c :: ls, h,  t)

  print cx (l, _, r) = (l, cx, r)
