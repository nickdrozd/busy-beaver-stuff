.PHONY : all clean clean-python compile coverage generate idris lint machines profile special test test-all type

all : machines idris lint test generate

clean-python :
	rm -rf __pycache__ **/__pycache__ .mypy_cache .coverage* htmlcov build/ *.so **/*.so

clean : clean-python
	$(MAKE) -C machines clean
	$(MAKE) -C idris clean

## Non-Python ##########################

machines :
	$(MAKE) -C machines

idris :
	$(MAKE) -C idris

## Python ##############################

PYTHON = python3

MODULES = tm generate test *.py

lint :
	$(PYTHON) -m pylint --version
	$(PYTHON) -m pylint --enable-all-extensions $(MODULES)
	$(MAKE) type

type :
	$(PYTHON) -m mypy --version
	$(PYTHON) -m mypy $(MODULES)

compile : clean-python
	$(PYTHON) -m mypyc tm generate --exclude tree

TUR = test.test_turing.Fast
PROG = test.test_program
GRAPH = test.test_graph
TREEF = test.test_tree.Fast
LR = test.test_lin_rado
CG = test.test_code
TP = test.test_tape

SHORT_TESTS = $(PROG) $(GRAPH) $(LR) $(TREEF) $(CG) $(TP) $(TUR)

PYTEST = $(PYTHON) -m unittest

test :
	$(PYTEST) -v $(SHORT_TESTS)

test-all : compile
	$(PYTEST) discover -v

COVERAGE = $(PYTHON) -m coverage

coverage : clean-python
	$(COVERAGE) --version
	$(COVERAGE) run -m unittest -v $(SHORT_TESTS)

	$(COVERAGE) combine --quiet
	$(COVERAGE) html

diagrams :
	pyreverse -o png tm generate test

# PYTHONPATH=$PYTHONPATH:tm make special target=tm/tape.py
special :
	specialist --target $(target) -m unittest $(TUR)

## Program files (non-phony) ###########

TIME = time -p
TREE = $(PYTHON) tree_gen.py

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
