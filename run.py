import sys

from tm.machine import Machine, opt_block

PRINT = 1
STEPS = 10 ** 10
PROVE = 1
BACKS = 0

PROFILE = 0

def main() -> None:
    for i, prog in enumerate(sys.stdin):
        program = prog.strip()

        machine = Machine(
            program,
            blocks = (
                opt
                if (opt := opt_block(program, steps = 8_000)) > 1 else
                None
            ),
            backsym = BACKS or None,
        ).run(
            sim_lim = STEPS,
            watch_tape = bool(PRINT),
            prover = bool(PROVE),
        )

        print(f'{i} | {machine}')

if __name__ == '__main__':
    if PROFILE:
        from perf import profile
        PRINT = 0
        main = profile(main)

    sys.set_int_max_str_digits(100_000)

    main()
