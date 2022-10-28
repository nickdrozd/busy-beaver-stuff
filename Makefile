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

MODULES = tm generate analyze test *.py

lint :
	pylint --version
	pylint $(MODULES)
	$(MAKE) type

type :
	mypy --version
	mypy $(MODULES)

compile : clean-python
	mypyc tm analyze generate/c.py generate/naive.py

TUR = test.test_turing.Fast
PROG = test.test_program
GRAPH = test.test_graph
GEN = test.test_generate
CG = test.test_code
TP = test.test_tape

SHORT_TESTS = $(PROG) $(GRAPH) $(GEN) $(CG) $(TP) $(TUR)

PYTEST = python3 -m unittest

test :
	$(PYTEST) -v $(SHORT_TESTS)

test-all : compile
	$(PYTEST) discover -v

COVERAGE = coverage

coverage : clean-python
	$(COVERAGE) --version
	$(COVERAGE) run -m unittest -v $(SHORT_TESTS)

	$(COVERAGE) combine --quiet
	$(COVERAGE) html

# PYTHONPATH=$PYTHONPATH:tm:analyze make special target=tm/tape.py
special :
	specialist --target $(target) -m unittest $(TUR)

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
