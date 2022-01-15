module Tape

import Data.List
import Data.Nat
import public Data.Vect
import public Data.Fin

import Program

%default total

public export
interface
TapeMeasure tape where
  cells : tape -> Nat
  marks : tape -> Nat

public export
Stepper : Type -> Type
Stepper tape = tape -> Color -> (Nat, tape)

public export
Shifter : Type -> Type
Shifter tape = Shift -> Stepper tape

public export
interface
Eq tape => TapeMeasure tape => Tape tape where
  blank : tape
  read  : tape -> (Color, Maybe Shift)

  step : Shifter tape
  step L = stepLeft
  step R = stepRight

  stepLeft  : Stepper tape
  stepRight : Stepper tape

public export
implementation
TapeMeasure tape => Show tape where
  show tape = show (cells tape, marks tape)

interface
Tape tape => MonotoneTape tape where
  tapeLengthMonotone : (cx : Color) -> (tp : tape) ->
    let (_, shifted) = stepLeft tp cx in
      LTE (cells tp) (cells shifted)

public export
interface
Tape tape => SkipTape tape where
  skip : Shifter tape
  skip L = skipLeft
  skip R = skipRight

  skipLeft  : Stepper tape
  skipLeft  = stepLeft

  skipRight : Stepper tape
  skipRight = stepRight

----------------------------------------

%hide span

interface
Eq span => Spannable span where
  pullNext : span -> (Color, span)
  pushCurr : Color -> Nat -> span -> span

  spanCells : span -> Nat
  spanMarks : span -> Nat

ScanNSpan : Type -> Type
ScanNSpan span = (span, Color, span)

implementation
Spannable span => TapeMeasure (ScanNSpan span) where
  cells (l, _, r) = spanCells l + 1 + spanCells r
  marks (l, c, r) = spanMarks l + (if c == 0 then 0 else 1) + spanMarks r

implementation
Spannable (List unit) => Tape (ScanNSpan (List unit)) where
  blank = ([], 0, [])

  read ([], c,  _) = (c,  Just L)
  read ( _, c, []) = (c,  Just R)
  read ( _, c,  _) = (c, Nothing)

  stepLeft (l, _, r) cx =
    let (x, k) = pullNext l in
      (1, (k, x, pushCurr cx 1 r))

  stepRight (l, c, r) cx =
    let (s, (k, x, e)) = stepLeft (r, c, l) cx in
      (s, (e, x, k))

implementation
Spannable (j : Nat ** Vect j unit) => Tape (ScanNSpan (k : Nat ** Vect k unit)) where
  blank = ((0 ** []), 0, (0 ** []))

  read ((_ ** []), c,         _) = (c,  Just L)
  read (        _, c, (_ ** [])) = (c,  Just R)
  read (        _, c,         _) = (c, Nothing)

  stepLeft (l, _, r) cx =
    let (x, k) = pullNext l in
      (1, (k, x, pushCurr cx 1 r))

  stepRight (l, c, r) cx =
    let (s, (k, x, e)) = stepLeft (r, c, l) cx in
      (s, (e, x, k))

----------------------------------------

public export
MicroTape : Type
MicroTape = ScanNSpan $ List Color

implementation
Spannable (List Color) where
  pullNext [] = (0, [])
  pullNext (x :: xs) = (x, xs)

  pushCurr cx _ xs = cx :: xs

  spanCells = length
  spanMarks = length . filter (/= 0)

public export
implementation
SkipTape MicroTape where
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

implementation
Spannable (List Block) where
  pullNext [] = (0, [])
  pullNext ((c, n) :: xs) =
    (c, case n of
             (S $ S k) => (c, S k) :: xs
             _         => xs)

  pushCurr c k [] = [(c, k)]
  pushCurr c k ((q, n) :: xs) =
    if c == q
      then (q, k + n) :: xs
      else (c, k) :: (q, n) :: xs

  spanCells = foldl (\a, (_, n) => a + n) 0
  spanMarks = foldl (\a, (q, n) => (+) a $ if q == 0 then 0 else n) 0

public export
MacroTape : Type
MacroTape = ScanNSpan $ List Block

public export
implementation
SkipTape MacroTape where
  skipLeft tape@([], _, _) cx = stepLeft tape cx

  skipLeft tape@(((bc, bn) :: l), c, r) cx =
    if bc /= c then stepLeft tape cx else
      let (x, k) = pullNext l in
        (1 + bn, (k, x, pushCurr cx (1 + bn) r))

  skipRight (l, c, r) cx =
    let (s, (k, x, e)) = skipLeft (r, c, l) cx in
      (s, (e, x, k))

----------------------------------------

public export
PtrTape : Type
PtrTape = (i : Nat ** (Fin (S i), Vect (S i) Color))

implementation
Eq PtrTape where
  (i1 ** (p1, t1)) == (i2 ** (p2, t2)) =
    (the Nat $ cast p1) == (the Nat $ cast p2)
      && toList t1 == toList t2

implementation
TapeMeasure PtrTape where
  cells (_ ** (_, tape))  = length tape
  marks (_ ** (_, tape)) = let (n ** _) = filter ((/=) 0) tape in n

public export
implementation
Tape PtrTape where
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
SkipTape PtrTape where
  skipLeft tape@(_ ** (FZ  , _)) cx = stepLeft tape cx

  skipLeft tape@(_ ** (FS _, _)) cx =
    let
      currScan = read tape
      oneStep@(_, stepped) = stepLeft tape cx
      nextScan = read stepped
    in
      if nextScan /= currScan then oneStep else
        let (steps, skipped) = assert_total $ skipLeft stepped cx in
          (S steps, skipped)

  skipRight tape cx =
    let
      currScan = read tape
      oneStep@(_, stepped) = stepRight tape cx
      nextScan = read stepped
    in
      if nextScan /= currScan then oneStep else
        let (steps, skipped) = assert_total $ skipRight stepped cx in
          (S steps, skipped)

implementation
MonotoneTape PtrTape where
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

----------------------------------------

implementation
Spannable Integer where
  spanCells = length . show
  spanMarks = length . filter ((/=) '0') . unpack . show

  pushCurr cx _ n = cast cx + (10 * n)

  pullNext n = (cast $ mod n 10, div n 10)

public export
NumTape : Type
NumTape = ScanNSpan Integer

public export
implementation
Tape NumTape where
  blank = (0, 0, 0)

  read (0, c, _) = (c,  Just L)
  read (_, c, 0) = (c,  Just R)
  read (_, c, _) = (c, Nothing)

  stepLeft (l, _, r) cx =
    let (x, k) = pullNext l in
      (1, (k, x, pushCurr cx 1 r))

  stepRight (l, c, r) cx =
    let (s, (k, x, e)) = stepLeft (r, c, l) cx in
      (s, (e, x, k))

public export
implementation
SkipTape NumTape where
  skipLeft tape@(0, _, _) cx = stepLeft tape cx

  skipLeft tape@(l, c, r) cx =
    let (cn, nl) = pullNext l in
      if c /= cn then stepLeft tape cx else
        let
          nextTape = (nl, cn, pushCurr cx 1 r)
          (steps, shifted) = assert_total $ skipLeft nextTape cx
        in
          (S steps, shifted)

  skipRight (l, c, r) cx =
    let (s, (k, x, e)) = skipLeft (r, c, l) cx in
      (s, (e, x, k))

----------------------------------------

VectSpan : Type -> Type
VectSpan unit = (k : Nat ** Vect k unit)

implementation
Eq ty => Eq (VectSpan ty) where
  (_ ** v1) == (_ ** v2) = toList v1 == toList v2

implementation
Spannable (VectSpan Color) where
  pullNext (S k ** c :: cs) = (c, (k ** cs))
  pullNext tape = (0, tape)

  pushCurr cx _ (k ** tape) = (S k ** cx :: tape)

  spanCells (_ ** tape) = length tape
  spanMarks (_ ** tape) = length $ filter (/= 0) $ toList tape

public export
MicroVectTape : Type
MicroVectTape = ScanNSpan $ VectSpan Color

public export
implementation
SkipTape MicroVectTape where
  skipLeft tape@((0 ** _), _, _) cx = stepLeft tape cx

  skipLeft tape@((S k ** cn :: l), c, (j ** r)) cx =
    if cn /= c then stepLeft tape cx else
      let
        nextTape = ((k ** l), cn, (S j ** cx :: r))
        (steps, shifted) = assert_total $ skipLeft nextTape cx
      in
        (S steps, shifted)

  skipRight (l, c, r) cx =
    let (s, (k, x, e)) = skipLeft (r, c, l) cx in
      (s, (e, x, k))

implementation
Spannable (VectSpan Block) where
  pullNext tape@(_ ** []) = (0, tape)
  pullNext (S j ** (c, n) :: xs) =
    (c, case n of
             (S $ S k) => (S j ** (c, S k) :: xs)
             _         => (j ** xs))

  pushCurr c k (0 ** []) = (1 ** [(c, k)])
  pushCurr c k (S j ** (q, n) :: xs) =
    if c == q
      then (S j ** (q, k + n) :: xs)
      else (S $ S j ** (c, k) :: (q, n) :: xs)

  spanCells (_ ** tape) = foldl (\a, (_, n) => a + n) 0 tape
  spanMarks (_ ** tape) = foldl (\a, (q, n) => (+) a $ if q == 0 then 0 else n) 0 tape

public export
MacroVectTape : Type
MacroVectTape = ScanNSpan $ VectSpan Block

public export
implementation
SkipTape MacroVectTape where
  skipLeft tape@((0 ** _), _, _) cx = stepLeft tape cx

  skipLeft tape@((S j ** ((bc, bn) :: l)), c, r) cx =
    if bc /= c then stepLeft tape cx else
      let (x, k) = pullNext $ the (VectSpan Block) (j ** l) in
        (1 + bn, (k, x, pushCurr cx (1 + bn) r))

  skipRight (l, c, r) cx =
    let (s, (k, x, e)) = skipLeft (r, c, l) cx in
      (s, (e, x, k))
----------------------------------------

Cast sp1 sp2 => Cast (ScanNSpan sp1) (ScanNSpan sp2) where
  cast (l, c, r) = (cast l, c, cast r)

Cast Block (List Color) where
  cast (c, n) = replicate n c

Cast (List Block) (List Color) where
  cast = concat . map cast

Cast (List Color) (List Block) where
  cast [] = []
  cast (c :: cs) =
    let
      block = (c, S $ length $ takeWhile (== c) cs)
      rest = assert_smaller cs $ dropWhile (== c) cs
    in
      block :: cast rest

Cast (List Color) Integer where
  cast [] = 0
  cast (c :: cs) = cast c + (10 * cast cs)

Cast Integer (List Color) where
  cast 0 = []
  cast n =
    let t = assert_smaller n $ div n 10 in
      cast (mod n 10) :: cast t

Cast Integer (List Block) where
  cast = cast . the (List Color) . cast

Cast (List Block) Integer where
  cast = cast . the (List Color) . cast
