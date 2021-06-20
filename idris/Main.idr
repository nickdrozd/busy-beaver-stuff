import BB
import Machine
import Program

%default total

partial
runBB : Program -> IO()
runBB prog = do
  -- let result = runOnBlankTape @{MicroMachine} prog
  -- putStrLn $ "*** Micro: " ++ show result

  let result = runOnBlankTape @{MacroMachine} prog
  putStrLn $ "*** Macro: " ++ show result

  pure()

partial
main : IO ()
main = do
  runBB BB2
  -- runBB BB3
  -- runBB BB4
  runBB tm5
  -- runBB TM24
  -- runBB BB24
  runBB bb5
