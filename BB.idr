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

BB3Literal : Vect 3 BWAction
BB3Literal = [
  [(1, (R, B)), (1, (R, H))],
  [(1, (L, B)), (0, (R, C))],
  [(1, (L, C)), (1, (L, A))]]

public export
BB3 : Program
BB3 = makeProgram BB3Literal

public export
bb3 : Program
bb3 A 0 = (1, R, B)
bb3 A 1 = (1, R, H)
bb3 B 0 = (1, L, B)
bb3 B 1 = (0, R, C)
bb3 C 0 = (1, L, C)
bb3 C 1 = (1, L, A)
bb3 _ c = (c, L, H)

-- 4/2

BB4Literal : Vect 4 BWAction
BB4Literal = [
  [(1, R, B), (1, L, B)],
  [(1, L, A), (0, L, C)],
  [(1, R, H), (1, L, D)],
  [(1, R, D), (0, R, A)]]

public export
BB4 : Program
BB4 = makeProgram BB4Literal

public export
bb4 : Program
bb4 A 0 = (1, R, B)
bb4 A 1 = (1, L, B)
bb4 B 0 = (1, L, A)
bb4 B 1 = (0, L, C)
bb4 C 0 = (1, R, H)
bb4 C 1 = (1, L, D)
bb4 D 0 = (1, R, D)
bb4 D 1 = (0, R, A)
bb4 _ c = (c, L, H)

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
