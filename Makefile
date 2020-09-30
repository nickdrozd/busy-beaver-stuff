all : lint test

run :
	python3 turing.py

lint :
	pylint *.py **/*.py

test :
	python3 -m unittest test.py

time : test

profile :
	python3 -m cProfile turing.py

clean :
	rm -rf yappi.* __pycache__

gen3 :
	python3 generate-3-state.py > 3-state-programs.txt
	wc -l 3-state-programs.txt

# make parse_graph prog="'1RB 1RC 1LC 1RD 1RA 1LD 0RD 0LB'"
parse_graph :
	python3 parse_graph.py $(prog) | dot -Tpng -o out.png
