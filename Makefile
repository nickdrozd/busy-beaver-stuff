.PHONY : all clean run lint test test-all profile machines idris tree

all : machines lint test

clean :
	rm -rf yappi.* __pycache__ **/__pycache__
	$(MAKE) -C machines clean
	$(MAKE) -C idris clean

run :
	python3 run.py

lint :
	pylint --version
	pylint *.py **/*.py

PYTEST = python3 -m unittest

test :
	$(PYTEST) -v test.test_turing.Fast

test-all :
	$(PYTEST) discover -v

profile :
	python3 -m cProfile run.py

machines :
	$(MAKE) -C machines

idris :
	$(MAKE) -C idris

tree :
	time -p python3 tree_gen.py
