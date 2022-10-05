import System
import Data.String

import BB
import Parse
import Machine
import Program

%default total

simLim : Nat
simLim = 128_000_000_000

checkResult : ProgData -> ProgData -> Bool -> Bool
checkResult (es, ec, em) (gs, gc, gm) checkCycles =
  es == gs && em == gm && (ec == gc || not checkCycles)

failWithMessage : String -> IO ()
failWithMessage msg = do putStrLn msg; exitFailure

failWhenWrong : String -> ProgData -> ProgData -> Bool -> IO ()
failWhenWrong prog expected actual checkCycles =
  unless (checkResult expected actual checkCycles) $ do
    putStrLn $ #"    Whoops!: \#{prog} | \#{show actual}"#

runProgram : Machine tp -> Program -> RunType -> IO $ Maybe (Steps, Cycles, tp)
runProgram machine prog Single =
  runOnBlankTape @{machine} simLim prog
runProgram machine prog DoubleRec =
  runDoubleOnBlank @{machine} simLim prog

runProgramGroup : Machine _ -> ProgramGroup -> Nat -> Bool -> IO ()
runProgramGroup _ (_, _, _, []) _ _ = putStrLn ""
runProgramGroup machine (n, k, rt, (prog, exp@(_, expCy, _)) :: rest) maxCycles checkCycles =
  if expCy > maxCycles
    then runProgramGroup machine (assert_smaller rest (n, k, rt, rest)) maxCycles checkCycles else do

  let Just parsed = parse n k prog
    | Nothing => failWithMessage $ "    Failed to parse: " ++ prog

  Just (steps, cycles, tape) <- runProgram machine parsed rt
    | Nothing => failWithMessage $ "    Hit limit: " ++ prog

  failWhenWrong prog exp (steps, cycles, marks tape) checkCycles

  putStrLn #"    \#{prog} | \#{show steps}"#

  runProgramGroup machine (assert_smaller rest (n, k, rt, rest)) maxCycles checkCycles

runProgramGroups : Machine _ -> List ProgramGroup -> Nat -> Bool -> IO ()
runProgramGroups _ [] _ _ = pure ()
runProgramGroups machine (progs :: rest) maxCycles checkCycles = do
  runProgramGroup machine progs maxCycles checkCycles
  runProgramGroups machine rest maxCycles checkCycles

runMachine : String -> Machine _ -> Nat -> IO ()
runMachine name machine maxCycles = do
  putStrLn $ "  " ++ name
  runProgramGroups machine testProgs maxCycles $ name /= "Ptr"

runPtr : IO ()
runPtr = runMachine "Ptr" PtrMachine 40_000

runNum : IO ()
runNum = runMachine "Num" NumMachine 40_000

runCell : IO ()
runCell = runMachine "Cell" CellMachine 3_000_000

runBlock : IO ()
runBlock = runMachine "Block" BlockMachine 100_000_000

runCellVect : IO ()
runCellVect = runMachine "CellVect" CellVectMachine 3_000_000

runBlockVect : IO ()
runBlockVect = runMachine "BlockVect" BlockVectMachine 100_000_000

runSlow : IO ()
runSlow = runMachine "Slow (Block)" BlockMachine 3_000_000_000

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
