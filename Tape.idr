module Tape

import Data.Nat
import Data.Vect

import Program

%default total

public export
Tape : Type
Tape = (posmax : Nat ** (Vect (S posmax) Color, Fin (S posmax)))

public export
readColor : Tape -> Color
readColor (_ ** (tape, pos)) = index pos tape

public export
printColor : Tape -> Color -> Tape
printColor (posmax ** (tape, pos)) color =
  (posmax ** (replaceAt pos color tape, pos))

public export
shiftHead : Tape -> Shift -> Tape
shiftHead (posmax ** (tape, pos)) shift =
  case shift of
    L => case pos of
      FZ   => (S posmax ** ([0] ++ tape, FZ))
      FS p => (  posmax ** (tape, weaken p))
    R => case pos of
      FZ   => case posmax of
           Z   => (S posmax ** (tape ++ [0], FS FZ))
           S _ => (  posmax ** (tape, FS FZ))
      FS _ => case strengthen pos of
           Right p => (  posmax ** (tape, FS p))
           Left  _ =>
             let prf = sym $ plusCommutative posmax 1 in
               (S posmax ** (rewrite prf in tape ++ [0], FS pos))
