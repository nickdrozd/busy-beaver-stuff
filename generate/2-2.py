from generate import yield_programs, print_programs

HALT = 1

if __name__ == '__main__':
    print_programs(
        yield_programs(
            2,
            2,
            halt=HALT))
