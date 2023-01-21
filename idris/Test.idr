import System
import Data.String

import BB
import Parse
import Machine
import Program

%default total

simLim : Cycles
simLim = 128_000_000_000

truncateProg : String -> String
truncateProg prog =
  if length prog < 50
    then prog
    else #"<... \#{show $ length prog} ...>"#

checkResult : ProgData -> ProgData -> Bool -> Bool
checkResult (es, ec, em) (gs, gc, gm) chkCy =
  es == gs && em == gm && (ec == gc || not chkCy)

failWithMessage : String -> IO ()
failWithMessage msg = do putStrLn msg; exitFailure

failWhenWrong : String -> ProgData -> ProgData -> Bool -> IO ()
failWhenWrong prog expected actual chkCy =
  unless (checkResult expected actual chkCy) $ do
    putStrLn $ #"    Whoops!: \#{prog} | \#{show actual}"#

runProg : Machine tp -> Program -> RunType -> IO $ Maybe (Steps, Cycles, tp)
runProg machine prog Single =
  runOnBlankTape @{machine} simLim prog
runProg machine prog DoubleRec =
  runDoubleOnBlank @{machine} simLim prog

runProgGroup : Machine _ -> Cycles -> Bool -> ProgramGroup -> IO ()
runProgGroup machine maxCy chkCy (n, k, rt, progs) =
  do traverse_ readAndRun progs; putStrLn ""
    where
  readAndRun : (String, ProgData) -> IO ()
  readAndRun (prog, exp@(_, expCy, _)) =
    when (maxCy > expCy) $ do
      let Just parsed = parse n k prog
        | Nothing => failWithMessage $ "    Failed to parse: " ++ prog

      Just (steps, cycles, tape) <- runProg machine parsed rt
        | Nothing => failWithMessage $ "    Hit limit: " ++ prog

      failWhenWrong prog exp (steps, cycles, marks tape) chkCy

      putStrLn #"    \#{truncateProg prog} | \#{show steps}"#

runMachine : String -> Machine _ -> Cycles -> IO ()
runMachine name machine maxCy = do
  putStrLn $ "  " ++ name
  traverse_ (runProgGroup machine maxCy (name /= "Ptr")) testProgs

runPtr : Cycles -> IO ()
runPtr = runMachine "Ptr" PtrMachine

runNum : Cycles -> IO ()
runNum = runMachine "Num" NumMachine

runCell : Cycles -> IO ()
runCell = runMachine "Cell" CellMachine

runFast : Cycles -> IO ()
runFast = runMachine "Block (Fast)" BlockMachine

runSplit : Cycles -> IO ()
runSplit = runMachine "Split" SplitMachine

runCV : Cycles -> IO ()
runCV = runMachine "CellVect" CellVectMachine

runBV : Cycles -> IO ()
runBV = runMachine "BlockVect" BlockVectMachine

runSlow : Cycles -> IO ()
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
  runSplit        100_000
  runBV        50_000_000

  runSlow   1_000_000_000
