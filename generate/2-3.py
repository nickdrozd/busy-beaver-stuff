from generate import yield_programs, print_programs

HALT = 0

if __name__ == '__main__':
    print_programs(
        yield_programs(
            2,
            3,
            HALT))
