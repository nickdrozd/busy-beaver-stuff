module Tape

import Data.List
import Data.Nat
import public Data.Vect
import public Data.Fin

import Program

%default total

public export
Stepper : Type -> Type
Stepper tape = tape -> Color -> (Nat, tape)

public export
Shifter : Type -> Type
Shifter tape = Shift -> Stepper tape

public export
interface
BasicTape tape where
  blank : tape
  read  : tape -> (Color, Maybe Shift)

  cells : tape -> Nat
  marks : tape -> Nat

  stepShift : Shifter tape
  stepShift L = stepLeft
  stepShift R = stepRight

  stepLeft  : Stepper tape
  stepRight : Stepper tape

public export
implementation
BasicTape tape => Show tape where
  show tape = show (cells tape, marks tape)

interface
BasicTape tape => MonotoneTape tape where
  tapeLengthMonotone : (cx : Color) -> (tp : tape) ->
    let (_, shifted) = stepLeft tp cx in
      LTE (cells tp) (cells shifted)

public export
interface
BasicTape tape => Tape tape where
  skipShift : Shifter tape
  skipShift L = skipLeft
  skipShift R = skipRight

  skipLeft  : Stepper tape
  skipLeft  = stepLeft

  skipRight : Stepper tape
  skipRight = stepRight

----------------------------------------

interface
Cast Color unit => ScanNSpan unit where
  pullNext : List unit -> (Color, List unit)
  pushCurr : unit -> List unit -> List unit

  spanCells : List unit -> Nat
  spanMarks : List unit -> Nat

implementation
ScanNSpan unit => BasicTape (List unit, Color, List unit) where
  blank = ([], 0, [])

  read ([], c,  _) = (c,  Just L)
  read ( _, c, []) = (c,  Just R)
  read ( _, c,  _) = (c, Nothing)

  cells (l, _, r) = spanCells l + 1 + spanCells r
  marks (l, c, r) = spanMarks l + (if c == 0 then 0 else 1) + spanMarks r

  stepLeft (l, _, r) cx =
    let (x, k) = pullNext l in
      (1, (k, x, pushCurr (cast cx) r))

  stepRight (l, c, r) cx =
    let (s, (k, x, e)) = stepLeft (r, c, l) cx in
      (s, (e, x, k))

----------------------------------------

public export
MicroTape : Type
MicroTape = (TapeSpan, Color, TapeSpan) where
  TapeSpan : Type
  TapeSpan = List Color

implementation
ScanNSpan Color where
  pullNext [] = (0, [])
  pullNext (x :: xs) = (x, xs)

  pushCurr = (::)

  spanCells = length
  spanMarks = length . filter (/= 0)

public export
implementation
Tape MicroTape where
  skipLeft tape@([], _, _) cx = stepLeft tape cx

  skipLeft tape@(cn :: l, c, r) cx =
    if cn /= c then stepLeft tape cx else
      let
        nextTape = (l, cn, cx :: r)
        (steps, shifted) = assert_total $ skipLeft nextTape cx
      in
        (S steps, shifted)

  skipRight (l, c, r) cx =
    let (s, (k, x, e)) = skipLeft (r, c, l) cx in
      (s, (e, x, k))

implementation
MonotoneTape MicroTape where
  tapeLengthMonotone _ (    [], _, _) =
    LTESucc $ lteSuccRight $ reflexive {rel = LTE}
  tapeLengthMonotone _ (_ :: t, _, r) =
    rewrite plusCommutative (length t) 1 in
      rewrite plusCommutative (length t) (S $ length r) in
        rewrite plusCommutative (length r) (length t) in
          LTESucc $ LTESucc $ reflexive {x = length t + length r}

----------------------------------------

Block : Type
Block = (Color, Nat)

Cast Color Block where
  cast c = (c, 1)

implementation
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
implementation
Tape MacroTape where
  skipLeft tape@([], _, _) cx = stepLeft tape cx

  skipLeft tape@(((bc, bn) :: l), c, r) cx =
    if bc /= c then stepLeft tape cx else
      let (x, k) = pullNext l in
        (1 + bn, (k, x, pushCurr (cx, 1 + bn) r))

  skipRight (l, c, r) cx =
    let (s, (k, x, e)) = skipLeft (r, c, l) cx in
      (s, (e, x, k))

----------------------------------------

public export
VLenTape : Type
VLenTape = (i : Nat ** (Fin (S i), Vect (S i) Color))

public export
implementation
BasicTape VLenTape where
  cells (_ ** (_, tape))  = length tape
  marks (_ ** (_, tape)) = let (n ** _) = filter ((/=) 0) tape in n

  blank = (Z ** (FZ, [0]))

  read (_ ** ( FZ, c :: _)) = (c, Just L)
  read (_ ** (pos,  tape )) =
    (index pos tape,
      case strengthen pos of
        Just _ => Nothing
        _      =>  Just R)

  stepLeft (i ** (pos, tape)) cx =
    let
      printed = replaceAt pos cx tape
      shifted =
        case pos of
          FZ   => (S i ** (FZ, [0] ++ printed))
          FS p => (  i ** ( weaken p, printed))
    in
      (1, shifted)

  stepRight (i ** (pos, tape)) cx =
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

public export
implementation
Tape VLenTape where

implementation
MonotoneTape VLenTape where
  tapeLengthMonotone _ (_ ** (FZ, _ :: _)) =
    lteSuccRight $ reflexive {rel = LTE}
  tapeLengthMonotone cx (S k ** (pos@(FS _), tape)) =
    rewrite replaceAtPresLen pos cx tape in
      reflexive {rel = LTE} where
    replaceAtPresLen :
      (p : Fin len) -> (e : ty) -> (v : Vect len ty)
      -> length (replaceAt p e v) = length v
    replaceAtPresLen _ _ [] impossible
    replaceAtPresLen FZ _ (_ :: _) = Refl
    replaceAtPresLen (FS p) e (_ :: ys) =
      rewrite replaceAtPresLen p e ys in
        Refl
