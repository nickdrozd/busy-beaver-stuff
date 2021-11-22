import BB
import Parse
import Machine
import Program

%default total

runPrograms : Machine _ -> Programs -> IO ()
runPrograms _ (_, _, []) = do putStrLn ""
runPrograms machine (n, k, (prog :: rest)) =
  case parse prog n k of
    Nothing => pure ()
    Just parsed => do
      result <- runOnBlankTape @{machine} parsed
      putStrLn $ "    " ++ prog ++ " | " ++ show result
      runPrograms machine (n, k, rest)

runProgramSets : Machine _ -> List Programs -> IO ()
runProgramSets _ [] = pure ()
runProgramSets machine (progs :: rest) = do
  runPrograms machine progs
  runProgramSets machine rest

Fast : List Programs
Fast = [p2_2, p3_2, p2_3, p4_2, p2_4, p5_2]

Slow : List Programs
Slow = [p3_3]

runMicro : IO ()
runMicro = do
  putStrLn "  Micro"
  runProgramSets MicroMachine Fast

runMacro : IO ()
runMacro = do
  putStrLn "  Macro"
  runProgramSets MacroMachine $ Fast ++ Slow

main : IO ()
main = do
  runMicro
  runMacro
