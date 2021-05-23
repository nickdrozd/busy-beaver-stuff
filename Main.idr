import BB
import Machine
import Program

%default total

partial
runBB : Program -> IO()
runBB prog = do
  let result = runOnBlankTape @{MicroMachine} prog
  putStrLn $ "*** " ++ show result
  pure()

partial
main : IO ()
main = do
  runBB BB2
  runBB BB3
  runBB bb3
  runBB BB4
  runBB bb4
  runBB tm5
