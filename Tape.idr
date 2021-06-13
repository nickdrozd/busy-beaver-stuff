module Tape

import Data.Nat
import public Data.Vect

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
TapeSpan = (n : Nat ** Vect n Color)

public export
MicroTape : Type
MicroTape = (TapeSpan, Color, TapeSpan)

public export
Tape MicroTape where
  cells ((l ** _), _, (r ** _)) = l + 1 + r

  marks ((_ ** l), c, (_ ** r)) =
    let (n ** _) = filter ((/=) 0) (l ++ [c] ++ r) in n

  blank = ((0 ** []), 0, (0 ** []))

  read (_, c, _) = c

  left  ((0 ** []), c, (r ** rs)) =
    ((0 ** []), 0, (S r ** c :: rs))

  left  ((S k ** h :: t), c, (r ** rs)) =
    ((k ** t), h, (S r ** c :: rs))

  right ((l ** ls), c, (0 ** [])) =
    ((S l ** c :: ls), 0, (0 ** []))

  right ((l ** ls), c, (S k ** h :: t)) =
    ((S l ** c :: ls), h, (k ** t))

  print cx (l, _, r) = (l, cx, r)
