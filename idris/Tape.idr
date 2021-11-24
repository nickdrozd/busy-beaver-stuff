module Tape

import Data.List
import Data.Nat

import Program

%default total

public export
interface Tape tape where
  cells : tape -> Nat
  marks : tape -> Nat

  blank : tape

  read  :          tape -> Color

  shift : Shift -> tape -> Color -> Bool -> (Nat, tape)
  shift L =  left
  shift R = right

  left  :          tape -> Color -> Bool -> (Nat, tape)
  right :          tape -> Color -> Bool -> (Nat, tape)

public export
Tape tape => Show tape where
  show tape = show (cells tape, marks tape)

----------------------------------------

interface ScanNSpan unit where
  pullNext : List unit -> (Color, List unit)
  pushCurr : unit -> List unit -> List unit

----------------------------------------

TapeSpan : Type
TapeSpan = List Color

public export
MicroTape : Type
MicroTape = (TapeSpan, Color, TapeSpan)

ScanNSpan Color where
  pullNext [] = (0, [])
  pullNext (x :: xs) = (x, xs)

  pushCurr = (::)

public export
Tape MicroTape where
  cells (l, _, r) = length l + 1 + length r

  marks (l, c, r) = length $ filter (/= 0) $ l ++ [c] ++ r

  blank = ([], 0, [])

  read (_, c, _) = c

  left tape@(cn :: l, c, r) cx True =
    if cn /= c then assert_total $ left tape cx False else
      let
        nextTape = (l, cn, cx :: r)
        (steps, shifted) = assert_total $ left nextTape cx True
      in
        (S steps, shifted)

  left (l, _, r) cx _ =
    let (x, k) = pullNext l in
      (1, (k, x, pushCurr cx r))

  right (l, c, r) cx skip =
    let (s, (k, x, e)) = left (r, c, l) cx skip in
      (s, (e, x, k))

tapeLengthMonotone : {x : Color} -> (tape : MicroTape) ->
  LTE (cells tape)
      (let (_, tp) = left tape x False in cells tp)
tapeLengthMonotone (    [], c, r) =
  LTESucc $ lteSuccRight $ reflexive {rel = LTE}
tapeLengthMonotone (h :: t, c, r) =
  rewrite plusCommutative (length t) 1 in
    rewrite plusCommutative (length t) (S $ length r) in
      rewrite plusCommutative (length r) (length t) in
        LTESucc $ LTESucc $ reflexive {x = length t + length r}

----------------------------------------

Block : Type
Block = (Color, Nat)

ScanNSpan Block where
  pullNext [] = (0, [])
  pullNext ((c, n) :: xs) =
    (c, case n of
             (S $ S k) => (c, S k) :: xs
             _         => xs)

  pushCurr block [] = [block]
  pushCurr block@(c, k) ((q, n) :: xs) =
    if c == q
      then (q, k + n) :: xs
      else block :: (q, n) :: xs

BlockSpan : Type
BlockSpan = List Block

public export
MacroTape : Type
MacroTape = (BlockSpan, Color, BlockSpan)

public export
Tape MacroTape where
  cells (l, _, r) =
    S $ foldl (\a, (_, n) => a + n) 0 $ l ++ r

  marks (l, c, r) =
    (+) (if c == 0 then 0 else 1) $
        foldl (\a, (q, n) =>
                   (+) a $ if q == 0 then 0 else n)
              0
              (l ++ r)

  blank = ([], 0, [])

  read (_, c, _) = c

  left tape@(((bc, bn) :: l), c, r) cx True =
    if bc /= c then assert_total $ left tape cx False else
      let (x, k) = pullNext l in
        (1 + bn, (k, x, pushCurr (cx, 1 + bn) r))

  left (l, _, r) cx _ =
    let (x, k) = pullNext l in
      (1, (k, x, pushCurr (cx, 1) r))

  right (l, c, r) cx skip =
    let (s, (k, x, e)) = left (r, c, l) cx skip in
      (s, (e, x, k))
