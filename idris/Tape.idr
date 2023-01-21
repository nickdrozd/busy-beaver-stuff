module Tape

import Data.List
import public Data.Nat
import public Data.Vect
import public Data.Fin

import Program

%default total

public export
interface
TapeMeasure tape where
  cells : tape -> Nat
  marks : tape -> Nat

  blank : tape -> Bool

  blankInit : tape

  read : tape -> (Color, Maybe Shift)


public export
Stepper : Type -> Type
Stepper tape = tape -> Color -> (Nat, tape)


public export
Shifter : Type -> Type
Shifter tape = Shift -> Stepper tape


public export
interface
Eq tape => TapeMeasure tape => Tape tape where
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
Tape tape => NonContractingTape tape where
  tapeNonContracting : (cx : Color) -> (tp : tape) ->
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

  blankSpan : span

  spanBlank : span -> Bool

  spanCells : span -> Nat
  spanMarks : span -> Nat


ScanNSpan : Type -> Type
ScanNSpan span = (span, Color, span)


implementation
Spannable span => TapeMeasure (ScanNSpan span) where
  cells (l, _, r) = spanCells l + 1 + spanCells r
  marks (l, c, r) = spanMarks l + (if c == 0 then 0 else 1) + spanMarks r

  blank (l, 0, r) = spanBlank l && spanBlank r
  blank _ = False

  blankInit = (blankSpan, 0, blankSpan)

  read (l, 0, r) =
    (0, if spanBlank l
           then Just L else
        if spanBlank r
          then Just R else
        Nothing)
  read (_, c, _) = (c, Nothing)


implementation
Spannable (List unit) => Tape (ScanNSpan (List unit)) where
  stepLeft (l, _, r) cx =
    let (x, k) = pullNext l in
      (1, (k, x, pushCurr cx 1 r))

  stepRight (l, c, r) cx =
    let (s, (k, x, e)) = stepLeft (r, c, l) cx in
      (s, (e, x, k))


implementation
Spannable (j : Nat ** Vect j unit) => Tape (ScanNSpan (k : Nat ** Vect k unit)) where
  stepLeft (l, _, r) cx =
    let (x, k) = pullNext l in
      (1, (k, x, pushCurr cx 1 r))

  stepRight (l, c, r) cx =
    let (s, (k, x, e)) = stepLeft (r, c, l) cx in
      (s, (e, x, k))

----------------------------------------

ColorSpan : Type
ColorSpan = List Color


public export
CellTape : Type
CellTape = ScanNSpan ColorSpan


implementation
Spannable ColorSpan where
  pullNext [] = (0, [])
  pullNext (x :: xs) = (x, xs)

  pushCurr cx _ xs = cx :: xs

  spanBlank (0 :: xs) = spanBlank xs
  spanBlank [] = True
  spanBlank _ = False

  blankSpan = []

  spanCells = length
  spanMarks = length . filter (/= 0)


public export
implementation
SkipTape CellTape where
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
NonContractingTape CellTape where
  tapeNonContracting _ (    [], _, _) =
    LTESucc $ lteSuccRight $ reflexive {rel = LTE}
  tapeNonContracting _ (_ :: t, _, r) =
    rewrite plusCommutative (length t) 1 in
      rewrite plusCommutative (length t) (S $ length r) in
        rewrite plusCommutative (length r) (length t) in
          LTESucc $ LTESucc $ reflexive {x = length t + length r}

----------------------------------------

Block : Type
Block = (Color, Nat)


BlockSpan : Type
BlockSpan = List Block


implementation
Spannable BlockSpan where
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

  blankSpan = []

  spanBlank ((0, _) :: xs) = spanBlank xs
  spanBlank [] = True
  spanBlank _ = False

  spanCells = foldl (\a, (_, n) => a + n) 0
  spanMarks = foldl (\a, (q, n) => (+) a $ if q == 0 then 0 else n) 0


public export
BlockTape : Type
BlockTape = ScanNSpan BlockSpan


public export
implementation
SkipTape BlockTape where
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

  blank (_ ** (_, cs)) = allZ cs where
    allZ : Vect _ Color -> Bool
    allZ [] = True
    allZ (0 :: xs) = allZ xs
    allZ _ = False

  blankInit = (0 ** (FZ, [0]))

  read (_ ** ( FZ, 0 :: _)) = (0,  Just L)
  read (_ ** ( FZ, c :: _)) = (c, Nothing)

  read tape@(S i ** (FS pos, 0 :: rest)) =
    read $ assert_smaller tape $
      the PtrTape (i ** (pos, rest))

  read (_ ** (pos,  tape )) =
    case index pos tape of
      0 => case strengthen pos of
        Just _ => (0, Nothing)
        _      => (0,  Just R)
      c => (c, Nothing)


public export
implementation
Tape PtrTape where
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
NonContractingTape PtrTape where
  tapeNonContracting _ (_ ** (FZ, _ :: _)) =
    lteSuccRight $ reflexive {rel = LTE}
  tapeNonContracting cx (S k ** (pos@(FS _), tape)) =
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

  spanBlank 0 = True
  spanBlank _ = False

  blankSpan = 0

  pushCurr cx _ n = cast cx + (10 * n)

  pullNext n = (cast $ mod n 10, div n 10)


public export
NumTape : Type
NumTape = ScanNSpan Integer


public export
implementation
Tape NumTape where
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

  spanBlank (_ ** cs) = allZ cs where
    allZ : Vect _ Color -> Bool
    allZ [] = True
    allZ (0 :: xs) = allZ xs
    allZ _ = False

  blankSpan = (0 ** [])

  spanCells (_ ** tape) = length tape
  spanMarks (_ ** tape) = length $ filter (/= 0) $ toList tape


public export
CellVectTape : Type
CellVectTape = ScanNSpan $ VectSpan Color


public export
implementation
SkipTape CellVectTape where
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

  spanBlank (_ ** bs) = allZ bs where
    allZ : Vect _ Block -> Bool
    allZ [] = True
    allZ ((0, _) :: xs) = allZ xs
    allZ _ = False

  blankSpan = (0 ** [])

  spanCells (_ ** tape) = foldl (\a, (_, n) => a + n) 0 tape
  spanMarks (_ ** tape) = foldl (\a, (q, n) => (+) a $ if q == 0 then 0 else n) 0 tape


public export
BlockVectTape : Type
BlockVectTape = ScanNSpan $ VectSpan Block


public export
implementation
SkipTape BlockVectTape where
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

Cast Block ColorSpan where
  cast (c, n) = replicate n c

Cast BlockSpan ColorSpan where
  cast = concat . map cast

Cast ColorSpan BlockSpan where
  cast [] = []
  cast (c :: cs) =
    let
      block = (c, S $ length $ takeWhile (== c) cs)
      rest = assert_smaller cs $ dropWhile (== c) cs
    in
      block :: cast rest

Cast ColorSpan Integer where
  cast [] = 0
  cast (c :: cs) = cast c + (10 * cast cs)

Cast Integer ColorSpan where
  cast 0 = []
  cast n =
    let t = assert_smaller n $ div n 10 in
      cast (mod n 10) :: cast t

Cast Integer BlockSpan where
  cast = cast . the ColorSpan . cast

Cast BlockSpan Integer where
  cast = cast . the ColorSpan . cast
