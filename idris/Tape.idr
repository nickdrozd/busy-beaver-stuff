module Tape

import Data.List
import Data.Nat
import public Data.Vect
import public Data.Fin

import Program

%default total

public export
interface BasicTape tape where
  blank : tape
  read  : tape -> Color

  cells : tape -> Nat
  marks : tape -> Nat

public export
interface BasicTape tape => Tape tape where
  shift : Shift -> tape -> Color -> Bool -> (Nat, tape)
  shift L =  left
  shift R = right

  left  :          tape -> Color -> Bool -> (Nat, tape)
  right :          tape -> Color -> Bool -> (Nat, tape)

public export
BasicTape tape => Show tape where
  show tape = show (cells tape, marks tape)

----------------------------------------

interface ScanNSpan unit where
  pullNext : List unit -> (Color, List unit)
  pushCurr : unit -> List unit -> List unit

  spanCells : List unit -> Nat
  spanMarks : List unit -> Nat

ScanNSpan unit => BasicTape (List unit, Color, List unit) where
  blank = ([], 0, [])

  read (_, c, _) = c

  cells (l, _, r) = spanCells l + 1 + spanCells r
  marks (l, c, r) = spanMarks l + (if c == 0 then 0 else 1) + spanMarks r

----------------------------------------

public export
MicroTape : Type
MicroTape = (TapeSpan, Color, TapeSpan) where
  TapeSpan : Type
  TapeSpan = List Color

ScanNSpan Color where
  pullNext [] = (0, [])
  pullNext (x :: xs) = (x, xs)

  pushCurr = (::)

  spanCells = length
  spanMarks = length . filter (/= 0)

public export
Tape MicroTape where
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

  spanCells = foldl (\a, (_, n) => a + n) 0
  spanMarks = foldl (\a, (q, n) => (+) a $ if q == 0 then 0 else n) 0

public export
MacroTape : Type
MacroTape = (BlockSpan, Color, BlockSpan) where
  BlockSpan : Type
  BlockSpan = List Block

public export
Tape MacroTape where
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

----------------------------------------

public export
VLenTape : Type
VLenTape = (i : Nat ** (Fin (S i), Vect (S i) Color))

public export
BasicTape VLenTape where
  cells (_ ** (_, tape))  = length tape
  marks (_ ** (_, tape)) = let (n ** _) = filter ((/=) 0) tape in n

  blank = (Z ** (FZ, [0]))

  read (_ ** (pos, tape)) = index pos tape

public export
Tape VLenTape where
  left (i ** (pos, tape)) cx _ =
    let
      printed = replaceAt pos cx tape
      shifted =
        case pos of
          FZ   => (S i ** (FZ, [0] ++ printed))
          FS p => (  i ** ( weaken p, printed))
    in
      (1, shifted)

  right (i ** (pos, tape)) cx _ =
    let
      printed = replaceAt pos cx tape
      shifted =
        case strengthen pos of
          Just p => (  i ** (FS p, printed))
          _      =>
            let prf = sym $ plusCommutative i 1 in
              (S i ** (FS pos, rewrite prf in printed ++ [0]))
    in
      (1, shifted)
