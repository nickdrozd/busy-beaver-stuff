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

runPrograms : Machine _ -> Programs -> IO Bool
runPrograms _ (_, _, []) = do putStrLn ""; pure False
runPrograms machine (n, k, (prog, expected) :: rest) = do
  let Just parsed = parse n k prog
    | Nothing => do
        putStrLn #"    Failed to parse: \#{prog}"#
        pure False

  Just (steps, tape) <- runOnBlankTape @{machine} xlimit parsed
    | Nothing => pure False

  let True = checkResult expected (steps, cells tape, marks tape)
    | _ => do
        putStrLn #"    Whoops! \#{prog} | should be \#{show (steps, tape)}"#
        pure False

  putStrLn #"    \#{prog} | \#{show steps}"#

  runPrograms machine $ assert_smaller rest (n, k, rest)

runProgramSets : Machine _ -> List Programs -> IO ()
runProgramSets _ [] = pure ()
runProgramSets machine (progs :: rest) = do
  _ <- runPrograms machine progs
  runProgramSets machine rest

Short : List Programs
Short = [p2_2, p3_2, p2_3, s4_2, s2_4, s5_2]

Mid : List Programs
Mid = [l4_2, l2_4, l5_2]

Long : List Programs
Long = [p6_2, p3_3]

runMachine : String -> Machine _ -> List Programs -> IO ()
runMachine name machine programs = do
  putStrLn $ "  " ++ name
  runProgramSets machine programs

runPtr : IO ()
runPtr = runMachine "Ptr" PtrMachine Short

runNum : IO ()
runNum = runMachine "Num" NumMachine Short

runMicro : IO ()
runMicro = runMachine "Micro" MicroMachine $ Short ++ Mid

runMacro : IO ()
runMacro = runMachine "Macro" MacroMachine $ Short ++ Mid ++ Long

runMicroVect : IO ()
runMicroVect = runMachine "MicroVect" MicroVectMachine $ Short ++ Mid

runMacroVect : IO ()
runMacroVect = runMachine "MacroVect" MacroVectMachine $ Short ++ Mid ++ Long

main : IO ()
main = do
  runPtr
  runNum
  runMicro
  runMacro
  runMicroVect
  runMacroVect
