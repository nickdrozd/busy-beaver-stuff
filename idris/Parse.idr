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
  s <- sat $ \x =>
           x == 'A' || x == 'B' || x == 'C' ||
           x == 'D' || x == 'E' || x == 'F' ||
           x == 'H'
  pure $ cast s

shift : Parser Shift
shift = do
  s <- sat $ \x => x == 'L' || x == 'R'
  pure $ cast s

color : Parser Color
color = do
  c <- sat isDigit
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

actions : (k : Nat) -> Parser $ Vect k Action
actions 0 = pure []
actions 1 = do i <- action; pure [i]
actions (S k) = do
  i  <- action
  _  <- space
  is <- actions k
  pure $ i :: is

program : (n, k : Nat) -> Parser $ Vect n $ Vect k Action
program 0 _ = pure []
program 1 c = do i <- actions c; pure [i]
program (S k) c = do
  i  <- actions c
  _  <- space
  _  <- space
  is <- program k c
  pure $ i :: is

colorIndex : Color -> Vect k Action -> Action
colorIndex _ [] = (1, R, H)
colorIndex 0 (i :: _) = i
colorIndex (S c) (_ :: is) = colorIndex c is

{n : Nat} -> Cast (Vect n $ Vect k Action) Program where
  cast prog state color =
    case n of
      1 =>
        case state of
          A => colorIndex color (index FZ prog)
          _ => (1, R, H)
      2 =>
        case state of
          A => colorIndex color $ index FZ prog
          B => colorIndex color $ index (FS FZ) prog
          _ => (1, R, H)
      3 =>
        case state of
          A => colorIndex color $ index FZ prog
          B => colorIndex color $ index (FS FZ) prog
          C => colorIndex color $ index (FS $ FS FZ) prog
          _ => (1, R, H)
      4 =>
        case state of
          A => colorIndex color $ index FZ prog
          B => colorIndex color $ index (FS FZ) prog
          C => colorIndex color $ index (FS $ FS FZ) prog
          D => colorIndex color $ index (FS $ FS $ FS FZ) prog
          _ => (1, R, H)
      5 =>
        case state of
          A => colorIndex color $ index FZ prog
          B => colorIndex color $ index (FS FZ) prog
          C => colorIndex color $ index (FS $ FS FZ) prog
          D => colorIndex color $ index (FS $ FS $ FS FZ) prog
          E => colorIndex color $ index (FS $ FS $ FS $ FS FZ) prog
          _ => (1, R, H)
      6 =>
        case state of
          A => colorIndex color $ index FZ prog
          B => colorIndex color $ index (FS FZ) prog
          C => colorIndex color $ index (FS $ FS FZ) prog
          D => colorIndex color $ index (FS $ FS $ FS FZ) prog
          E => colorIndex color $ index (FS $ FS $ FS $ FS FZ) prog
          F => colorIndex color $ index (FS $ FS $ FS $ FS $ FS FZ) prog
          _ => (1, R, H)
      _ => (1, R, H)

public export
parse : (n, k : Nat) -> String -> Maybe Program
parse n k prog = do
  parsed <- runParser (program n k) prog
  Just $ cast parsed
