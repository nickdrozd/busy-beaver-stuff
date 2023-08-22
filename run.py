import sys

from tm.machine import Machine

PRINT = 1
STEPS = 10 ** 10
BACKS = 0

PROFILE = 0

def main() -> None:
    for i, prog in enumerate(sys.stdin):
        program = prog.strip()

        machine = Machine(
            program,
            opt_macro = 8_000,
            backsym = BACKS or None,
        ).run(
            sim_lim = STEPS,
            watch_tape = bool(PRINT),
        )

        print(f'{i} | {machine}')

if __name__ == '__main__':
    if PROFILE:
        from perf import profile
        PRINT = 0
        main = profile(main)

    sys.set_int_max_str_digits(100_000)

    main()
