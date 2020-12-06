HOLDOUTS = {
    # Lot 1
    0o73037233,
    0o73137233,
    0o73137123,
    0o73136523,
    0o73133271,
    0o73133251,
    0o73132742,
    0o73132542,
    0o73032532,
    0o73032632,
    0o73033132,
    0o73033271,
    0o73073271,
    0o73075221,
    # Lot 2
    0o73676261,
    0o73736122,
    0o71536037,
    0o73336333,
    0o71676261,
    0o73336133,
    0o73236333,
    0o73236133,
    # Lot 3
    0o70537311,
    0o70636711,
    0o70726711,
    0o72737311,
    0o71717312,
    0o72211715,
    0o72237311,
    0o72311715,
    0o72317716,
    0o72331715,
    0o72337311,
    0o72337315,
    # Lot 4
    0o70513754,
    0o70612634,
    0o70712634,
    0o72377034,
    0o72377234,
    0o72613234,
}


def oct_to_bin(oct_string):
    return '{0:b}'.format(oct_string)


def bin_to_prog(bin_string):
    instrs = [
        bin_string[i : i + 4]
        for i in range(0, len(bin_string), 4)

    ]

    return ' '.join(map(convert_bin_instr, instrs))


def convert_bin_instr(bin_instr):
    pr, sh, *tr =  bin_instr

    tr = int(''.join(tr), 2)

    return '{}{}{}'.format(
        pr,
        'L' if int(sh) == 0 else 'R',
        'H' if tr == 0 else chr(tr + 64),
    )


def convert(rado_string):
    return bin_to_prog(
        oct_to_bin(
            rado_string))
