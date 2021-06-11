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

public export
MicroTape : Type
MicroTape = (i : Nat ** (Fin (S i), Vect (S i) Color))

public export
Tape MicroTape where
  cells (_ ** (_, tape))  = length tape
  marks (_ ** (_, tape)) = let (n ** _) = filter ((/=) 0) tape in n

  blank = (Z ** (FZ, [0]))

  read (_ ** (pos, tape)) =
    index pos tape

  print color (i ** (pos, tape)) =
    (i ** (pos, replaceAt pos color tape))

  left (i ** (FZ,   tape)) = (S i ** (FZ, [0] ++ tape))
  left (i ** (FS p, tape)) = (  i ** ( weaken p, tape))

  right (i ** (pos, tape)) =
    case strengthen pos of
      Right p => (  i ** (FS p, tape))
      Left  _ =>
        let prf = sym $ plusCommutative i 1 in
          (S i ** (FS pos, rewrite prf in tape ++ [0]))

----------------------------------------

MacroBlock : Type
MacroBlock = (Color, (j : Nat ** Fin (S j)))

public export
MacroTape : Type
MacroTape = (i : Nat ** (Fin (S i), Vect (S i) MacroBlock))

data SplitBlock
  = Replaced MacroBlock
  | SplitBeg MacroBlock MacroBlock
  | SplitMid MacroBlock MacroBlock MacroBlock
  | SplitEnd MacroBlock MacroBlock

splitPrint : Color -> MacroBlock -> SplitBlock
splitPrint cx (c0, coord) =
  case coord of
    (Z ** FZ) =>
      Replaced (cx, (Z ** FZ))

    (S j ** FZ) =>
      SplitBeg (cx, (Z ** FZ)) (c0, (j ** FZ))

    (S j ** FS pos) =>
      case strengthen (FS pos) of
        Left  _ =>
          SplitEnd (c0, (j ** pos)) (cx, (Z ** FZ))

        Right p =>
          case splitPrint cx (c0, (j ** p)) of
            Replaced x =>
              SplitEnd (c0, (0 ** FZ)) x

            SplitBeg   x c =>
              SplitMid (c0, (0 ** FZ)) x c

            SplitMid (_, (k ** q)) x c =>
              SplitMid (c0, (S k ** FS q)) x c

            SplitEnd (_, (k ** q)) x =>
              SplitEnd (c0, (S k ** FS q)) x


mergeRight : MacroBlock -> MacroBlock -> Maybe MacroBlock
mergeRight c@(cc, (cj ** cp)) r@(rc, (rj ** rp)) =
  if cc /= rc then Nothing else Just $
    let pos = weakenLTE cp $ lteAddRight (S cj) {m = S rj} in
      (cc, (cj + S rj ** pos))

mergeLeft : MacroBlock -> MacroBlock -> Maybe MacroBlock
mergeLeft l@(lc, (lj ** lp)) c@(cc, (cj ** cp)) =
  if lc /= cc then Nothing else Just $
    (cc, (lj + S cj ** shift (S lj) cp))

splitMerge : Color -> (Maybe MacroBlock) -> MacroBlock -> (Maybe MacroBlock) -> MacroTape

splitMerge xc Nothing c Nothing =
  case splitPrint xc c of
    Replaced   x   => (0 ** (FZ, [x]))
    SplitBeg   x b => (1 ** (FZ, [x, b]))
    SplitMid a x b => (2 ** (FS FZ, [a, x, b]))
    SplitEnd a x   => (1 ** (FS FZ, [a, x]))

splitMerge xc Nothing c@(cc, (cj ** cp)) (Just r@(rc, (rj ** rp))) =
  case splitPrint xc c of
    SplitBeg   x b => (2 ** (FZ, [x, b, r]))
    SplitMid a x b => (3 ** (FS FZ, [a, x, b, r]))
    Replaced   x   =>
      case mergeRight x r of
        Just m  => (0 ** (FZ, [m]))
        Nothing => (1 ** (FZ, [x, r]))
    SplitEnd a x   =>
      case mergeRight x r of
        Just m  => (1 ** (FS FZ, [a, m]))
        Nothing => (2 ** (FS FZ, [a, x, r]))

splitMerge xc (Just l@(lc, (lj ** lp))) c@(cc, (cj ** cp)) Nothing  =
  case splitPrint xc c of
    SplitMid a x b => (3 ** (FS $ FS FZ, [l, a, x, b]))
    SplitEnd a x   => (2 ** (FS $ FS FZ, [l, a, x]))
    Replaced   x   =>
      case mergeLeft l x of
        Just m  => (0 ** (FZ, [m]))
        Nothing => (1 ** (FS FZ, [l, x]))
    SplitBeg   x b =>
      case mergeLeft l x of
        Just m  => (1 ** (FZ, [m, b]))
        Nothing => (2 ** (FS FZ, [l, x, b]))

splitMerge xc (Just l@(lc, (lj ** lp))) c@(cc, (cj ** cp)) (Just r@(rc, (rj ** rp))) =
  case splitPrint xc c of
    SplitMid a x b => (4 ** (FS $ FS FZ, [l, a, x, b, r]))
    SplitEnd a x   =>
      case mergeRight x r of
        Just m  => (2 ** (FS $ FS FZ, [l, a, m]))
        Nothing => (3 ** (FS $ FS FZ, [l, a, x, r]))
    SplitBeg   x b =>
      case mergeLeft l x of
        Just m  => (2 ** (FZ, [m, b, r]))
        Nothing => (3 ** (FS FZ, [l, x, b, r]))
    Replaced   x   =>
      case mergeLeft l x of
        Just ml =>
          case mergeRight ml r of
            Just mr => (0 ** (FZ, [mr]))
            Nothing => (1 ** (FZ, [ml, r]))
        Nothing =>
          case mergeRight x r of
            Just mr => (1 ** (FS FZ, [l, mr]))
            Nothing => (2 ** (FS FZ, [l, x, r]))

public export
Tape MacroTape where
  cells (_ ** (_, blocks)) =
    foldl (\acc, (_, (i ** _)) => S i + acc) 0 blocks

  marks (_ ** (_, blocks)) =
    foldl (\acc, (c, (i ** _)) =>
                 (if c == 0 then 0 else S i) + acc) 0 blocks

  blank = (0 ** (FZ, [(0, (0 ** FZ))]))

  read (_ ** (blockIndex, blocks)) =
    let (color, _) = index blockIndex blocks in
      color

  ----------------------------------------

  print cx (0 ** (   FZ, [b0])) =
    splitMerge cx Nothing b0 Nothing

  print cx (1 ** (   FZ, [b0, b1])) =
    splitMerge cx Nothing b0 (Just b1)

  print cx (1 ** (FS FZ, [b0, b1])) =
    splitMerge cx (Just b0) b1 Nothing

  print cx (S $ S k ** (   FZ, b0 :: b1 :: rest)) =
    let
      (j ** (pos, blocks)) = splitMerge cx Nothing b0 (Just b1)
      adj = weakenLTE pos $ lteAddRight (S j) {m = S k}
    in
      (j + S k ** (adj, blocks ++ rest))

  print cx (S $ S k ** (FS FZ, b0 :: b1 :: b2 :: rest)) =
    let
      (j ** (pos, blocks)) = splitMerge cx (Just b0) b1 (Just b2)
      adj = weakenLTE pos $ lteAddRight (S j) {m = k}
    in
      (j + k ** (adj, blocks ++ rest))

  print cx (S $ S k ** (FS $ FS p, b0 :: rest)) =
    let
      tail = the MacroTape (S k ** (FS p, rest))
      (j ** (pos, blocks)) = print cx tail
    in
      (S j ** (FS pos, b0 :: blocks))

  ----------------------------------------

  right (0 ** (FZ, [(c, (j ** pos))])) =
    case strengthen pos of
      Right p => (0 ** (FZ, [(c, (j ** FS p))]))
      Left  p =>
        case c of
          0 => (0 ** (FZ, [(0, (S j ** FS p))]))
          _ => (1 ** (FS FZ, (c, (j ** pos)) :: [(0, (0 ** FZ))]))

  right (S i ** (FZ, (c, (j ** pos)) :: blocks)) =
    case strengthen pos of
      Right p => (S i ** (FZ, (c, (j ** FS p)) :: blocks))
      Left  _ =>
        let (q, (k ** _)) :: rest = blocks in
          (S i ** (FS FZ, (c, (j ** pos)) :: (q, (k **  FZ)) :: rest))

  right (S i ** (FS p, b0 :: rest)) =
    let
      tail = the MacroTape (i ** (p, rest))
      (j ** (pos, blocks)) = right tail
    in
      (S j ** (FS pos, b0 :: blocks))

  ----------------------------------------

  left (0 ** (FZ, block@[(S _, (j ** FZ))])) =
    (1 ** (FZ, (0, (0 ** FZ)) :: block))
  left (S i ** (FZ, blocks@((S _, (j ** FZ)) :: rest))) =
    (S $ S i ** (FZ, (0, (0 ** FZ)) :: blocks))

  left (0 ** (FZ, (c, (j ** FS p)) :: rest)) =
    (0 ** (FZ, (c, (j ** weaken p)) :: rest))
  left (S i ** (FZ, (c, (j ** FS p)) :: rest)) =
    (S i ** (FZ, (c, (j ** weaken p)) :: rest))

  left (S i ** (FS FZ, (c, (j ** _)) :: b1@(_, (_ ** FZ)) :: rest)) =
    (S i ** (FZ, (c, (j ** last)) :: b1 :: rest))

  left (S i ** (FS FZ, b0 :: (c, (j ** FS p)) :: rest)) =
    (S i ** (FS FZ, b0 :: (c, (j ** weaken p)) :: rest))

  left (S i ** (FS $ FS p, block :: rest)) =
    let
      tail = the MacroTape (i ** (FS p, rest))
      (k ** (pos, blocks)) = left tail
    in
      (S k ** (FS pos, block :: blocks))

  left (i ** (FZ, (0, (j ** FZ)) :: rest)) =
    (i ** (FZ, (0, (S j ** FZ)) :: rest))
