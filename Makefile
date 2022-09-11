.PHONY : all clean coverage machines idris lint test test-all run profile generate

all : machines idris lint test generate

clean :
	rm -rf yappi.* __pycache__ **/__pycache__ .mypy_cache .coverage htmlcov
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
COVERAGE = coverage run -m unittest

TUR = test.test_turing.Fast
PROG = test.test_program
GRAPH = test.test_graph
QTREE = test.test_generate.TestTree.test_22
LR = test.test_generate.TestLinRado.test_22
NV = test.test_generate.TestNaive.test_22

TEST = $(PROG) $(GRAPH) $(QTREE) $(LR) $(NV) $(TUR)

test :
	$(PYTEST) -v $(TEST)

test-all :
	$(PYTEST) discover -v

coverage :
	$(COVERAGE) -v $(TEST)

	coverage combine --quiet
	coverage html

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
	$(TIME) $(TREE) 5 2 > $@

generate : 3-2.prog 2-3.prog
	wc -l *.prog

CALLGRIND = yappi.callgrind
PROFILE = yappi.png

$(PROFILE) : $(CALLGRIND)
	gprof2dot $(CALLGRIND) -f callgrind --colour-nodes-by-selftime | dot -T png -o $(PROFILE)

profile : $(PROFILE)
