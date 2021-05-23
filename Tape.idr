module Tape

import Data.Nat
import Data.Vect

import Program

%default total

public export
interface Tape tape where
  readColor  : tape -> Color
  printColor : tape -> Color -> tape
  shiftHead  : tape -> Shift -> tape

public export
MicroTape : Type
MicroTape = (posmax : Nat ** (Vect (S posmax) Color, Fin (S posmax)))

public export
Tape MicroTape where
  readColor (_ ** (tape, pos)) =
    index pos tape

  printColor (posmax ** (tape, pos)) color =
    (posmax ** (replaceAt pos color tape, pos))

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
