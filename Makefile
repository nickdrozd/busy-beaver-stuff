.PHONY : all clean  machines idris lint test test-all run profile tree

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

PYPATH = PYTHONPATH=.:$PYTHONPATH

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

tree :
	time -p python3 tree_gen.py
