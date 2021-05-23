import Data.Vect

import Machine
import Parse

%default total

partial
main : IO ()
main = do
  let result = runOnBlankTape @{MicroMachine} tm5
  putStrLn $ "\n*** " ++ show result
  pure ()
