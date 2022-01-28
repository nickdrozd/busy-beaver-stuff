module Parse

import Data.Vect
import Data.String

import Program

%default total

-- https://timmyjose.github.io/docs/2020-09-19-parser-combinator-library-idris.html

data Parser : Type -> Type where
  MkParser : (String -> List (a, String)) -> Parser a

Functor Parser where
  map f (MkParser p) = MkParser $ \inp =>
    case p inp of
      [(v, out)] => [(f v, out)]
      _          => []

Applicative Parser where
  pure v = MkParser $ \inp => [(v, inp)]

  (MkParser pf) <*> p = MkParser $ \inp =>
    case pf inp of
      [(f, out)] => let (MkParser fp) = (map f p) in fp out
      _          => []

Monad Parser where
  (MkParser p) >>= f = MkParser $ \inp =>
    case p inp of
      [(v, out)] => let (MkParser fv) = f v in fv out
      _          => []

Alternative Parser where
  empty = MkParser $ \_ => []

  (MkParser p) <|> (MkParser q) = MkParser $ \inp =>
    case p inp of
      [(v, out)] => [(v, out)]
      _          => q inp

runParser : Parser a -> String -> Maybe a
runParser (MkParser p) inp =
  case p inp of
    [(res, "")] => Just res
    _           => Nothing

----------------------------------------

item : Parser Char
item = MkParser (\inp =>
  case (unpack inp) of
    [] => []
    (c :: cs) => [(c, pack cs)])

sat : (Char -> Bool) -> Parser Char
sat p = do
  x <- item
  if p x then pure x else empty

state : Parser State
state = do
  s <- sat $ \x => isUpper x || x == '_' || x == '.'
  pure $ cast @{CastState} s

shift : Parser Shift
shift = do
  s <- sat $ \x => x == 'L' || x == 'R' || x == '.'
  pure $ cast s

color : Parser Color
color = do
  c <- sat $ \x => isDigit x || x == '.'
  pure $ stringToNatOrZ $ pack [c]

action : Parser Action
action = do
  co <- color
  sh <- shift
  st <- state
  pure (co, sh, st)

space : Parser ()
space = do
  _ <- sat isSpace
  pure ()

ColorVect : Nat -> Type
ColorVect k = Vect k Action

ProgVect : (n, k : Nat) -> Type
ProgVect n k = Vect n $ ColorVect k

actions : (k : Nat) -> Parser $ ColorVect k
actions 0 = pure []
actions 1 = do i <- action; pure [i]
actions (S k) = do
  i  <- action
  _  <- space
  is <- actions k
  pure $ i :: is

program : (n, k : Nat) -> Parser $ ProgVect n k
program 0 _ = pure []
program 1 c = do i <- actions c; pure [i]
program (S k) c = do
  i  <- actions c
  _  <- space
  _  <- space
  is <- program k c
  pure $ i :: is

colorIndex : Color -> ColorVect k -> Action
colorIndex _ [] = (1, R, halt)
colorIndex 0 (i :: _) = i
colorIndex (S c) (_ :: is) = colorIndex c is

{n : Nat} -> Cast (ProgVect n k) Program where
  cast prog state color =
    case n of
      1 =>
        case state of
          1 => colorIndex color (index FZ prog)
          _ => (1, R, halt)
      2 =>
        case state of
          1 => colorIndex color $ index FZ prog
          2 => colorIndex color $ index (FS FZ) prog
          _ => (1, R, halt)
      3 =>
        case state of
          1 => colorIndex color $ index FZ prog
          2 => colorIndex color $ index (FS FZ) prog
          3 => colorIndex color $ index (FS $ FS FZ) prog
          _ => (1, R, halt)
      4 =>
        case state of
          1 => colorIndex color $ index FZ prog
          2 => colorIndex color $ index (FS FZ) prog
          3 => colorIndex color $ index (FS $ FS FZ) prog
          4 => colorIndex color $ index (FS $ FS $ FS FZ) prog
          _ => (1, R, halt)
      5 =>
        case state of
          1 => colorIndex color $ index FZ prog
          2 => colorIndex color $ index (FS FZ) prog
          3 => colorIndex color $ index (FS $ FS FZ) prog
          4 => colorIndex color $ index (FS $ FS $ FS FZ) prog
          5 => colorIndex color $ index (FS $ FS $ FS $ FS FZ) prog
          _ => (1, R, halt)
      6 =>
        case state of
          1 => colorIndex color $ index FZ prog
          2 => colorIndex color $ index (FS FZ) prog
          3 => colorIndex color $ index (FS $ FS FZ) prog
          4 => colorIndex color $ index (FS $ FS $ FS FZ) prog
          5 => colorIndex color $ index (FS $ FS $ FS $ FS FZ) prog
          6 => colorIndex color $ index (FS $ FS $ FS $ FS $ FS FZ) prog
          _ => (1, R, halt)
      7 =>
        case state of
          1 => colorIndex color $ index FZ prog
          2 => colorIndex color $ index (FS FZ) prog
          3 => colorIndex color $ index (FS $ FS FZ) prog
          4 => colorIndex color $ index (FS $ FS $ FS FZ) prog
          5 => colorIndex color $ index (FS $ FS $ FS $ FS FZ) prog
          6 => colorIndex color $ index (FS $ FS $ FS $ FS $ FS FZ) prog
          7 => colorIndex color $ index (FS $ FS $ FS $ FS $ FS $ FS FZ) prog
          _ => (1, R, halt)
      8 =>
        case state of
          1 => colorIndex color $ index FZ prog
          2 => colorIndex color $ index (FS FZ) prog
          3 => colorIndex color $ index (FS $ FS FZ) prog
          4 => colorIndex color $ index (FS $ FS $ FS FZ) prog
          5 => colorIndex color $ index (FS $ FS $ FS $ FS FZ) prog
          6 => colorIndex color $ index (FS $ FS $ FS $ FS $ FS FZ) prog
          7 => colorIndex color $ index (FS $ FS $ FS $ FS $ FS $ FS FZ) prog
          8 => colorIndex color $ index (FS $ FS $ FS $ FS $ FS $ FS $ FS FZ) prog
          _ => (1, R, halt)
      9 =>
        case state of
          1 => colorIndex color $ index FZ prog
          2 => colorIndex color $ index (FS FZ) prog
          3 => colorIndex color $ index (FS $ FS FZ) prog
          4 => colorIndex color $ index (FS $ FS $ FS FZ) prog
          5 => colorIndex color $ index (FS $ FS $ FS $ FS FZ) prog
          6 => colorIndex color $ index (FS $ FS $ FS $ FS $ FS FZ) prog
          7 => colorIndex color $ index (FS $ FS $ FS $ FS $ FS $ FS FZ) prog
          8 => colorIndex color $ index (FS $ FS $ FS $ FS $ FS $ FS $ FS FZ) prog
          9 => colorIndex color $ index (FS $ FS $ FS $ FS $ FS $ FS $ FS $ FS FZ) prog
          _ => (1, R, halt)
      10 =>
        case state of
          1 => colorIndex color $ index FZ prog
          2 => colorIndex color $ index (FS FZ) prog
          3 => colorIndex color $ index (FS $ FS FZ) prog
          4 => colorIndex color $ index (FS $ FS $ FS FZ) prog
          5 => colorIndex color $ index (FS $ FS $ FS $ FS FZ) prog
          6 => colorIndex color $ index (FS $ FS $ FS $ FS $ FS FZ) prog
          7 => colorIndex color $ index (FS $ FS $ FS $ FS $ FS $ FS FZ) prog
          8 => colorIndex color $ index (FS $ FS $ FS $ FS $ FS $ FS $ FS FZ) prog
          9 => colorIndex color $ index (FS $ FS $ FS $ FS $ FS $ FS $ FS $ FS FZ) prog
          10 => colorIndex color $ index (FS $ FS $ FS $ FS $ FS $ FS $ FS $ FS $ FS FZ) prog
          _ => (1, R, halt)
      11 =>
        case state of
          1 => colorIndex color $ index FZ prog
          2 => colorIndex color $ index (FS FZ) prog
          3 => colorIndex color $ index (FS $ FS FZ) prog
          4 => colorIndex color $ index (FS $ FS $ FS FZ) prog
          5 => colorIndex color $ index (FS $ FS $ FS $ FS FZ) prog
          6 => colorIndex color $ index (FS $ FS $ FS $ FS $ FS FZ) prog
          7 => colorIndex color $ index (FS $ FS $ FS $ FS $ FS $ FS FZ) prog
          8 => colorIndex color $ index (FS $ FS $ FS $ FS $ FS $ FS $ FS FZ) prog
          9 => colorIndex color $ index (FS $ FS $ FS $ FS $ FS $ FS $ FS $ FS FZ) prog
          10 => colorIndex color $ index (FS $ FS $ FS $ FS $ FS $ FS $ FS $ FS $ FS FZ) prog
          11 => colorIndex color $ index (FS $ FS $ FS $ FS $ FS $ FS $ FS $ FS $ FS FZ) prog
          _ => (1, R, halt)
      12 =>
        case state of
          1 => colorIndex color $ index FZ prog
          2 => colorIndex color $ index (FS FZ) prog
          3 => colorIndex color $ index (FS $ FS FZ) prog
          4 => colorIndex color $ index (FS $ FS $ FS FZ) prog
          5 => colorIndex color $ index (FS $ FS $ FS $ FS FZ) prog
          6 => colorIndex color $ index (FS $ FS $ FS $ FS $ FS FZ) prog
          7 => colorIndex color $ index (FS $ FS $ FS $ FS $ FS $ FS FZ) prog
          8 => colorIndex color $ index (FS $ FS $ FS $ FS $ FS $ FS $ FS FZ) prog
          9 => colorIndex color $ index (FS $ FS $ FS $ FS $ FS $ FS $ FS $ FS FZ) prog
          10 => colorIndex color $ index (FS $ FS $ FS $ FS $ FS $ FS $ FS $ FS $ FS FZ) prog
          11 => colorIndex color $ index (FS $ FS $ FS $ FS $ FS $ FS $ FS $ FS $ FS FZ) prog
          12 => colorIndex color $ index (FS $ FS $ FS $ FS $ FS $ FS $ FS $ FS $ FS $ FS FZ) prog
          _ => (1, R, halt)
      _ => (1, R, halt)

public export
parse : (n, k : Nat) -> String -> Maybe Program
parse n k prog = do
  parsed <- runParser (program n k) prog
  Just $ cast parsed
