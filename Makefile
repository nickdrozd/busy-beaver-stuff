.PHONY : all clean  machines idris lint test test-all run profile generate

all : machines lint test

clean :
	rm -rf yappi.* __pycache__ **/__pycache__ .mypy_cache
	$(MAKE) -C machines clean
	$(MAKE) -C idris clean

## Non-Python ##########################

machines :
	$(MAKE) -C machines

idris :
	$(MAKE) -C idris

## Python ##############################

PYPATH = PYTHONPATH=.:$(PYTHONPATH)

lint :
	pylint --version
	pylint *.py **/*.py

	mypy --version
	mypy tm generate test

PYTEST = python3 -m unittest

test :
	$(PYTEST) -v test.test_turing.Fast
	$(PYTEST) -v test.test_program
	$(PYTEST) -v test.test_graph
	$(PYTEST) -v test.test_generate.TestLinRado.test_22h

test-all :
	$(PYTEST) discover -v

run :
	$(PYPATH) python3 bin/run.py

profile :
	$(PYPATH) python3 -m cProfile bin/run.py

## Program files (non-phony) ###########

TIME = time -p
TREE = python3 bin/tree_gen.py

3-2.prog :
	$(PYPATH) $(TIME) $(TREE) 3 2 | sort > $@

2-3.prog :
	$(PYPATH) $(TIME) $(TREE) 2 3 | sort > $@

4-2.prog :
	# $(PYPATH) $(TIME) $(TREE) 4 2 | sort > $@

generate : 3-2.prog 2-3.prog
	wc -l *.prog
