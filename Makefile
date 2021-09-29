.PHONY : all run lint test test-all time profile clean machines idris tree

all : machines structured lint test

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

time : test

profile :
	python3 -m cProfile run.py

clean :
	rm -rf yappi.* __pycache__ **/__pycache__ **/run
	$(MAKE) -C machines clean

machines :
	$(MAKE) -C machines

structured :
	$(MAKE) -C machines/structured

idris :
	$(MAKE) -C idris

tree :
	time -p python3 tree_gen.py
