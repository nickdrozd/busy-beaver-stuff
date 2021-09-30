.PHONY : all clean  machines idris lint test test-all run profile generate

all : machines lint test

clean :
	rm -rf yappi.* __pycache__ **/__pycache__
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

PYTEST = python3 -m unittest

test :
	$(PYTEST) -v test.test_turing.Fast

test-all :
	$(PYTEST) discover -v

run :
	$(PYPATH) python3 bin/run.py

profile :
	$(PYPATH) python3 -m cProfile bin/run.py

## Program files (non-phony) ###########

3-2.prog :
	$(PYPATH) time -p python3 bin/tree_gen.py 3 > $@

4-2.prog :
	$(PYPATH) time -p python3 bin/tree_gen.py 4 > $@

generate : 3-2.prog 4-2.prog
