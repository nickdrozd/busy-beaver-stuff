module Tape

import Data.Nat
import public Data.Vect

import Program

%default total

public export
interface Show tape => Tape tape where
  blank : tape

  read  :          tape -> Color
  print : Color -> tape -> tape

  shift : Shift -> tape -> tape
  shift L tape =  left tape
  shift R tape = right tape

  left  :          tape -> tape
  right :          tape -> tape

----------------------------------------

public export
MicroTape : Type
MicroTape = (i : Nat ** (Vect (S i) Color, Fin (S i)))

public export
Show MicroTape where
  show (_ ** (tape, _)) = show (length tape, marks tape) where
    marks : Vect k Color -> Nat
    marks xs = let (n ** _) = filter ((/=) 0) xs in n

public export
Tape MicroTape where
  blank = (Z ** ([0], FZ))

  read (_ ** (tape, pos)) =
    index pos tape

  print color (i ** (tape, pos)) =
    (i ** (replaceAt pos color tape, pos))

  left (i ** (tape,   FZ)) = (S i ** ([0] ++ tape, FZ))
  left (i ** (tape, FS p)) = (  i ** (tape, weaken p))

  right (  Z ** (tape,   FZ)) = (S Z ** (tape ++ [0], FS FZ))
  right (S i ** (tape,   FZ)) = (S i ** (tape       , FS FZ))

  right (i ** (tape, FS p)) =
    case strengthen $ FS p of
      Right p => (  i ** (tape, FS p))
      Left  _ =>
        let prf = sym $ plusCommutative i 1 in
          (S i ** (rewrite prf in tape ++ [0], FS $ FS p))
