#!/bin/bash

prog_file="prog-4-2.txt"

python3 generate-4-2.py > $prog_file

wc -l $prog_file

programs=(
    "1RB 1RC 1LC 1RD 1RA 1LD 0RD 0LB"
    "1RB 0LC 1LD 0LA 1RC 1RD 1LA 0LD"
    "1RB 1RA 0RC 0RB 0RD 1RA 1LD 1LB"
    "1RB 1RA 0RC 1LA 1LC 1LD 0RB 0RD"
)

for prog in "${programs[@]}"; do
    if ! grep "$prog" $prog_file; then
        echo "missing: $prog"
        exit 1;
    fi
done
