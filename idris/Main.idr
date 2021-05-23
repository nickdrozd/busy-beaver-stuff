import Data.Vect

import Machine
import Parse
import Tape

%default total

partial
main : IO ()
main = do
  let result = runOnBlankTape tm5
  putStrLn $ "\n*** " ++ show (the (Nat, MicroTape) result)
  pure ()
