.PHONY : all clean compile coverage generate idris lint machines profile test test-all

all : machines idris lint test generate

clean :
	rm -rf yappi.* __pycache__ **/__pycache__ .mypy_cache .coverage htmlcov build/ *.so **/*.so
	$(MAKE) -C machines clean
	$(MAKE) -C idris clean

## Non-Python ##########################

machines :
	$(MAKE) -C machines

idris :
	$(MAKE) -C idris

## Python ##############################

MODULES = tm generate analyze test *.py

lint :
	pylint --version
	pylint $(MODULES)

	mypy --version
	mypy $(MODULES)

compile :
	mypyc tm

TUR = test.test_turing.Fast
PROG = test.test_program
GRAPH = test.test_graph
CG = test.test_c
QTREE = test.test_generate.TestTree.test_22
LR = test.test_generate.TestLinRado.test_22
NV = test.test_generate.TestNaive.test_22
TP = test.test_tape

SHORT_TESTS = $(PROG) $(GRAPH) $(CG) $(QTREE) $(LR) $(NV) $(TP) $(TUR)

PYTEST = python3 -m unittest

test :
	$(PYTEST) -v $(SHORT_TESTS)

test-all : compile
	$(PYTEST) discover -v

COVERAGE = coverage

coverage :
	$(COVERAGE) run -m unittest -v $(SHORT_TESTS)

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
