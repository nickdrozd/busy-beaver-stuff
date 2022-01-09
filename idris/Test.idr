import System

import BB
import Parse
import Machine
import Program

%default total

xlimit : Nat
xlimit = 1_000_000_000

checkResult : (Nat, Nat, Nat) -> (Nat, Nat, Nat) -> Bool
checkResult (es, _, em) (gs, _, gm) =
  es == gs && em == gm

failWithMessage : String -> IO ()
failWithMessage msg = do putStrLn msg; exitFailure

failWhenWrong : String -> (Nat, Nat, Nat) -> (Nat, Nat, Nat) -> IO ()
failWhenWrong prog expected actual =
  unless (checkResult expected actual) $ do
    failWithMessage $ #"    Whoops!: \#{prog} | \#{show actual}"#

runProgram : Machine tp -> Program -> RunType -> IO $ Maybe (Nat, tp)
runProgram machine prog Single =
  runOnBlankTape @{machine} xlimit prog
runProgram machine prog DoubleRec =
  runDoubleOnBlank @{machine} xlimit prog

runPrograms : Machine _ -> Programs -> IO ()
runPrograms _ (_, _, _, []) = putStrLn ""
runPrograms machine (n, k, rt, (prog, expected) :: rest) = do
  let Just parsed = parse n k prog
    | Nothing => failWithMessage $ "    Failed to parse: " ++ prog

  Just (steps, tape) <- runProgram machine parsed rt
    | Nothing => failWithMessage $ "    Hit limit: " ++ prog

  failWhenWrong prog expected (steps, cells tape, marks tape)

  putStrLn #"    \#{prog} | \#{show steps}"#

  runPrograms machine $ assert_smaller rest (n, k, rt, rest)

runProgramSets : Machine _ -> List Programs -> IO ()
runProgramSets _ [] = pure ()
runProgramSets machine (progs :: rest) = do
  runPrograms machine progs
  runProgramSets machine rest

Short : List Programs
Short = [p2_2, p3_2, p2_3, s4_2, s2_4, s5_2]

Rec : List Programs
Rec = [d3_2, d2_3, d2_4]

Mid : List Programs
Mid = [l4_2, l2_4, l5_2, p8_4, p5_5, p7_7]

Long : List Programs
Long = [p6_2, p3_3, ll2_4]

LongLong : List Programs
LongLong = [p6_9]

runMachine : String -> Machine _ -> List Programs -> IO ()
runMachine name machine programs = do
  putStrLn $ "  " ++ name
  runProgramSets machine programs

runPtr : IO ()
runPtr = runMachine "Ptr" PtrMachine Short

runNum : IO ()
runNum = runMachine "Num" NumMachine $ Short ++ Rec

runMicro : IO ()
runMicro = runMachine "Micro" MicroMachine $ Short ++ Rec ++ Mid

runMacro : IO ()
runMacro = runMachine "Macro" MacroMachine $ Short ++ Rec ++ Mid ++ Long

runMicroVect : IO ()
runMicroVect = runMachine "MicroVect" MicroVectMachine $ Short ++ Rec ++ Mid

runMacroVect : IO ()
runMacroVect = runMachine "MacroVect" MacroVectMachine $ Short ++ Rec ++ Mid ++ Long

main : IO ()
main = do
  runPtr
  runNum
  runMicro
  runMacro
  runMicroVect
  runMacroVect
