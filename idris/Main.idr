import BB
import Machine
import Program

%default total

FastPrograms : List Program
FastPrograms = [BB2, BB3, BB4, tm5, TM24, BB24, bb5]

SlowPrograms : List Program
SlowPrograms = [TM33F, TM33S, TM33Q]

runPrograms : Machine _ -> List Program -> IO ()
runPrograms _ [] = do putStrLn ""
runPrograms machine (prog :: rest) = do
  let result = runOnBlankTape @{machine} prog
  putStrLn $ "    " ++ show result
  runPrograms machine rest

partial
runMicro : IO ()
runMicro = do
  putStrLn "  Micro"
  runPrograms MicroMachine FastPrograms

partial
runMacro : IO ()
runMacro = do
  putStrLn "  Macro"
  runPrograms MacroMachine FastPrograms
  runPrograms MacroMachine SlowPrograms

partial
main : IO ()
main = do
  runMicro
  runMacro
