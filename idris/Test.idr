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

runPtr : Nat -> IO ()
runPtr = runMachine "Ptr" PtrMachine

runNum : Nat -> IO ()
runNum = runMachine "Num" NumMachine

runCell : Nat -> IO ()
runCell = runMachine "Cell" CellMachine

runFast : Nat -> IO ()
runFast = runMachine "Block (Fast)" BlockMachine

runCV : Nat -> IO ()
runCV = runMachine "CellVect" CellVectMachine

runBV : Nat -> IO ()
runBV = runMachine "BlockVect" BlockVectMachine

runSlow : Nat -> IO ()
runSlow = runMachine "Block (Slow)" BlockMachine

main : IO ()
main = do
  runFast      10_000_000

  let [_, _] = stringToNatOrZ <$> !getArgs
    | _ => do putStrLn "Skipping slow tests...\n"; exitSuccess

  runPtr            5_000
  runNum            5_000
  runCell         100_000
  runCV           100_000
  runBV        50_000_000

  runSlow   3_000_000_000
