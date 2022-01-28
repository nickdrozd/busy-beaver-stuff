.PHONY : all clean  machines idris lint test test-all run profile generate

all : machines idris lint test generate

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
	$(PYPATH) python3 run.py

profile :
	python3 -m cProfile run.py

## Program files (non-phony) ###########

TIME = time -p
TREE = python3 tree_gen.py

3-2.prog :
	$(TIME) $(TREE) 3 2 | sort > $@

2-3.prog :
	$(TIME) $(TREE) 2 3 | sort > $@

4-2.prog :
	$(TIME) $(TREE) 4 2 | tee $@
	sort -o $@ $@

2-4.prog :
	$(TIME) $(TREE) 2 4 | tee $@
	sort -o $@ $@

5-2.prog :
	$(TIME) $(TREE) 5 2 | tee $@
	sort -o $@ $@

generate : 3-2.prog 2-3.prog
	wc -l *.prog
