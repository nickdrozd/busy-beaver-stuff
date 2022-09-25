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
	mypy tm generate analyze

TUR = test.test_turing.Fast
PROG = test.test_program
GRAPH = test.test_graph
CG = test.test_c
QTREE = test.test_generate.TestTree.test_22
LR = test.test_generate.TestLinRado.test_22
NV = test.test_generate.TestNaive.test_22
TP = test.test_tape

MODULES = $(PROG) $(GRAPH) $(CG) $(QTREE) $(LR) $(NV) $(TP) $(TUR)

PYTEST = python3 -m unittest

test :
	$(PYTEST) -v $(MODULES)

test-all :
	$(PYTEST) discover -v

COVERAGE = coverage

coverage :
	$(COVERAGE) run -m unittest -v $(MODULES)

	$(COVERAGE) combine --quiet
	$(COVERAGE) html

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
