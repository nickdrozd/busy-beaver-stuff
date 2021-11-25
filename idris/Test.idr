import BB
import Parse
import Machine
import Program

%default total

checkResult : (Nat, Nat, Nat) -> (Nat, Nat, Nat) -> Bool
checkResult (es, el, em) (gs, gl, gm) =
  es == gs && el == gl && em == gm

runPrograms : Machine _ -> Programs -> IO Bool
runPrograms _ (_, _, []) = do putStrLn ""; pure False
runPrograms machine (n, k, (prog, expected) :: rest) = do
  let Just parsed = parse prog n k
    | Nothing => do
        putStrLn #"    Failed to parse: \#{prog}"#
        pure False

  (steps, tape) <- runOnBlankTape @{machine} parsed

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
Short = [p2_2, p3_2, p2_3]

Mid : List Programs
Mid = [p4_2, p2_4, p5_2]

Long : List Programs
Long = [p6_2, p3_3]

runVLen : IO ()
runVLen = do
  putStrLn "  VLen"
  runProgramSets VLenMachine Short

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
  runVLen
  runMicro
  runMacro
