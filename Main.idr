import BB
import Machine
import Program

%default total

Programs : Type
Programs = List Program

FastHalt : Programs
FastHalt = [BB2, BB3, BB4, tm5, TM24, BB24, bb5]

SlowHalt : Programs
SlowHalt = [TM33F, TM33S, TM33Q]

Blankers : Programs
Blankers = [BL2, BL3, BL4]

runPrograms : Machine _ -> Programs -> IO ()
runPrograms _ [] = do putStrLn ""
runPrograms machine (prog :: rest) = do
  result <- runOnBlankTape @{machine} prog
  putStrLn $ "    " ++ show result
  runPrograms machine rest

runProgramSets : Machine _ -> List Programs -> IO ()
runProgramSets _ [] = pure ()
runProgramSets machine (progs :: rest) = do
  runPrograms machine progs
  runProgramSets machine rest

partial
runMicro : IO ()
runMicro = do
  putStrLn "  Micro"
  runProgramSets MicroMachine [
    FastHalt,
    Blankers]

partial
runMacro : IO ()
runMacro = do
  putStrLn "  Macro"
  runProgramSets MacroMachine [
    FastHalt,
    SlowHalt,
    Blankers]

partial
main : IO ()
main = do
  runMicro
  runMacro
