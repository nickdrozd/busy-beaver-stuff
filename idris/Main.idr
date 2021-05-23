import BB
import Machine

%default total

partial
main : IO ()
main = do
  let result = runOnBlankTape @{MicroMachine} tm5
  putStrLn $ "\n*** " ++ show result
  pure ()
