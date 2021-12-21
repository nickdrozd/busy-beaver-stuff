module Run

import Parse
import Machine
import Program

xlimit : Nat
xlimit = 1_000_000_000

main : IO ()
main = loop 1 where
  loop : Nat -> IO ()
  loop i = do
    putStrLn $ show i

    prog <- getLine

    if prog == ""
      then putStrLn "done"
      else do
        let Just parsed = parse 2 4 prog
          | Nothing => do
            putStrLn #"    Failed to parse: \#{prog}"#
            pure ()

        Just (steps, _) <- runOnBlankTape @{MacroMachine} xlimit parsed
                    | Nothing => loop $ S i

        putStrLn #"    \#{prog} | \#{show steps}"#

        loop $ S i
