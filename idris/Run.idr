module Run

import System
import Data.String

import Parse
import Machine
import Program

simLim : Nat
simLim = 100_000_000

main : IO ()
main = do
  let [_, states, colors] = stringToNatOrZ <$> !getArgs
    | _ => do putStrLn "Couldn't parse args"; exitFailure
  loop 1 (states, colors)
    where
  loop : Nat -> (Nat, Nat) -> IO ()
  loop i params@(states, colors) = do
    putStrLn $ show i

    prog <- getLine

    if prog == ""
      then putStrLn "done"
      else do
        let Just parsed = parse states colors prog
          | Nothing => do
            putStrLn #"    Failed to parse: \#{prog} (\#{show states}, \#{show colors})"#
            pure ()

        Just (steps, _) <- runOnBlankTape @{MacroMachine} simLim parsed
                    | Nothing => loop (S i) params

        putStrLn #"    \#{prog} | \#{show steps}"#

        loop (S i) params
