import System
import Data.String

import BB
import Parse
import Machine
import Program

%default total

simLim : Nat
simLim = 128_000_000_000

checkResult : (Nat, Nat, Nat) -> (Nat, Nat, Nat) -> Bool -> Bool
checkResult (es, ec, em) (gs, gc, gm) checkCycles =
  es == gs && em == gm && (ec == gc || True)

failWithMessage : String -> IO ()
failWithMessage msg = do putStrLn msg; exitFailure

failWhenWrong : String -> (Nat, Nat, Nat) -> (Nat, Nat, Nat) -> Bool -> IO ()
failWhenWrong prog expected actual checkCycles =
  unless (checkResult expected actual checkCycles) $ do
    putStrLn $ #"    Whoops!: \#{prog} | \#{show actual}"#

runProgram : Machine tp -> Program -> RunType -> IO $ Maybe (Steps, Cycles, tp)
runProgram machine prog Single =
  runOnBlankTape @{machine} simLim prog
runProgram machine prog DoubleRec =
  runDoubleOnBlank @{machine} simLim prog

runPrograms : Machine _ -> Programs -> Bool -> IO ()
runPrograms _ (_, _, _, []) _ = putStrLn ""
runPrograms machine (n, k, rt, (prog, expected) :: rest) checkCycles = do
  let Just parsed = parse n k prog
    | Nothing => failWithMessage $ "    Failed to parse: " ++ prog

  Just (steps, cycles, tape) <- runProgram machine parsed rt
    | Nothing => failWithMessage $ "    Hit limit: " ++ prog

  failWhenWrong prog expected (steps, cycles, marks tape) checkCycles

  putStrLn #"    \#{prog} | \#{show steps}"#

  runPrograms machine (assert_smaller rest (n, k, rt, rest)) checkCycles

runProgramSets : Machine _ -> List Programs -> Bool -> IO ()
runProgramSets _ [] _ = pure ()
runProgramSets machine (progs :: rest) checkCycles = do
  runPrograms machine progs checkCycles
  runProgramSets machine rest checkCycles

Short : List Programs
Short = [p2_2, p3_2, p2_3, s4_2, s2_4, s5_2]

Rec : List Programs
Rec = [d3_2, d2_3, d2_4]

Mid : List Programs
Mid = [l4_2, l2_4, l5_2, p8_4, p5_5, p7_7]

Long : List Programs
Long = [p6_2, p3_3, ll2_4, ll5_2]

LongLong : List Programs
LongLong = [lll5_2, p6_9, lll2_4, ll9_8]

runMachine : String -> Machine _ -> List Programs -> IO ()
runMachine name machine programs = do
  putStrLn $ "  " ++ name
  runProgramSets machine programs $ name /= "Ptr"

runPtr : IO ()
runPtr = runMachine "Ptr" PtrMachine Short

runNum : IO ()
runNum = runMachine "Num" NumMachine $ Short ++ Rec

runCell : IO ()
runCell = runMachine "Cell" CellMachine $ Short ++ Rec ++ Mid

runBlock : IO ()
runBlock = runMachine "Block" BlockMachine $ Short ++ Rec ++ Mid ++ Long

runCellVect : IO ()
runCellVect = runMachine "CellVect" CellVectMachine $ Short ++ Rec ++ Mid

runBlockVect : IO ()
runBlockVect = runMachine "BlockVect" BlockVectMachine $ Short ++ Rec ++ Mid ++ Long

runSlow : IO ()
runSlow = runMachine "Slow (Block)" BlockMachine LongLong

main : IO ()
main = do
  runBlock

  let [_, _] = stringToNatOrZ <$> !getArgs
    | _ => do putStrLn "Skipping slow tests...\n"; exitSuccess

  runPtr
  runNum
  runCell
  runCellVect
  runBlockVect

  runSlow
