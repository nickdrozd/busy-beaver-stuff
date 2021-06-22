module BB

import Data.Vect

import Program
import public Parse

%default total

-- 2/2

public export
BB2 : Program
BB2 A 0 = (1, R, B)
BB2 A 1 = (1, L, B)
BB2 B 0 = (1, L, A)
BB2 B 1 = (1, R, H)
BB2 _  c = (c, L, H)

-- 3/2

bb3Literal : Vect 3 BWAction
bb3Literal = [
  [(1, (R, B)), (1, (R, H))],
  [(1, (L, B)), (0, (R, C))],
  [(1, (L, C)), (1, (L, A))]]

public export
bb3 : Program
bb3 = makeProgram bb3Literal

public export
BB3 : Program
BB3 A 0 = (1, R, B)
BB3 A 1 = (1, R, H)
BB3 B 0 = (1, L, B)
BB3 B 1 = (0, R, C)
BB3 C 0 = (1, L, C)
BB3 C 1 = (1, L, A)
BB3 _ c = (c, L, H)

-- 4/2

bb4Literal : Vect 4 BWAction
bb4Literal = [
  [(1, R, B), (1, L, B)],
  [(1, L, A), (0, L, C)],
  [(1, R, H), (1, L, D)],
  [(1, R, D), (0, R, A)]]

public export
bb4 : Program
bb4 = makeProgram bb4Literal

public export
BB4 : Program
BB4 A 0 = (1, R, B)
BB4 A 1 = (1, L, B)
BB4 B 0 = (1, L, A)
BB4 B 1 = (0, L, C)
BB4 C 0 = (1, R, H)
BB4 C 1 = (1, L, D)
BB4 D 0 = (1, R, D)
BB4 D 1 = (0, R, A)
BB4 _ c = (c, L, H)

-- 5/2

public export
tm5 : Program
tm5 = makeProgram [
  [(1, (R, B)), (0, (L, C))],
  [(1, (R, C)), (1, (R, D))],
  [(1, (L, A)), (0, (R, B))],
  [(0, (R, E)), (1, (R, H))],
  [(1, (L, C)), (1, (R, A))]]

public export
bb5 : Program
bb5 = makeProgram [
  [(1, (R, B)), (1, (L, C))],
  [(1, (R, C)), (1, (R, B))],
  [(1, (R, D)), (0, (L, E))],
  [(1, (L, A)), (1, (L, D))],
  [(1, (R, H)), (0, (L, A))]]

-- 2/4

public export
TM24 : Program
TM24 A 0 = (1, R, B)
TM24 A 1 = (3, L, A)
TM24 A 2 = (1, L, A)
TM24 A 3 = (1, R, A)
TM24 B 0 = (2, L, A)
TM24 B 1 = (1, R, H)
TM24 B 2 = (3, R, A)
TM24 B 3 = (3, R, B)
TM24 _ c = (c, L, H)

public export
BB24 : Program
BB24 A 0 = (1, R, B)
BB24 A 1 = (2, L, A)
BB24 A 2 = (1, R, A)
BB24 A 3 = (1, R, A)
BB24 B 0 = (1, L, B)
BB24 B 1 = (1, L, A)
BB24 B 2 = (3, R, B)
BB24 B 3 = (1, R, H)
BB24 _ c = (c, L, H)

public export
TM33 : Program
TM33 A 0 = (1, R, B)
TM33 A 1 = (1, R, H)
TM33 A 2 = (2, R, B)
TM33 B 0 = (1, L, C)
TM33 B 1 = (0, L, B)
TM33 B 2 = (1, R, A)
TM33 C 0 = (1, R, A)
TM33 C 1 = (2, L, C)
TM33 C 2 = (1, R, C)
TM33 _ c = (c, L, H)
