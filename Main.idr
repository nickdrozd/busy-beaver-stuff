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
  let result = runOnBlankTape @{machine} prog
  putStrLn $ "    " ++ show result
  runPrograms machine rest

partial
runMicro : IO ()
runMicro = do
  putStrLn "  Micro"
  runPrograms MicroMachine FastHalt

partial
runMacro : IO ()
runMacro = do
  putStrLn "  Macro"
  runPrograms MacroMachine FastHalt
  runPrograms MacroMachine SlowHalt

partial
main : IO ()
main = do
  runMicro
  runMacro
