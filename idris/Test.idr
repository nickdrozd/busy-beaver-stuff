import BB
import Parse
import Machine
import Program

%default total

checkResult : (Nat, Nat, Nat) -> (Nat, Nat, Nat) -> Bool
checkResult (es, _, em) (gs, _, gm) =
  es == gs && em == gm

runPrograms : Machine _ -> Programs -> IO Bool
runPrograms _ (_, _, []) = do putStrLn ""; pure False
runPrograms machine (n, k, (prog, expected) :: rest) = do
  let Just parsed = parse prog n k
    | Nothing => do
        putStrLn #"    Failed to parse: \#{prog}"#
        pure False

  Just (steps, tape) <- runOnBlankTape @{machine} parsed
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

runPtr : IO ()
runPtr = do
  putStrLn "  Ptr"
  runProgramSets PtrMachine Short

runNum : IO ()
runNum = do
  putStrLn "  Num"
  runProgramSets NumMachine Short

runMicro : IO ()
runMicro = do
  putStrLn "  Micro"
  runProgramSets MicroMachine $ Short ++ Mid

runMacro : IO ()
runMacro = do
  putStrLn "  Macro"
  runProgramSets MacroMachine $ Short ++ Mid ++ Long

main : IO ()
main = do
  runPtr
  runNum
  runMicro
  runMacro
