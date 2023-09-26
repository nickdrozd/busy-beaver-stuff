.PHONY : all clean clean-python compile coverage generate idris lint machines profile special test test-all type

all : machines idris lint test generate

clean-python :
	rm -rf __pycache__ **/__pycache__ .mypy_cache .mutmut-cache .coverage* html* build/ *.so **/*.so classes.png packages.png

clean : clean-python clean-rust
	$(MAKE) -C machines clean
	$(MAKE) -C idris clean

refresh : clean-python rust

quick-check : refresh lint coverage

## Odd langs ###########################

machines :
	$(MAKE) -C machines

idris :
	$(MAKE) -C idris

## Rust ################################

RUST_STUFF = tm/rust_stuff.so

$(RUST_STUFF) :
	cargo build --release
	cp target/release/librust_stuff.so $(RUST_STUFF)

rust :
	cargo build --release
	cp target/release/librust_stuff.so $(RUST_STUFF)

clippy :
	cargo --version
	cargo clippy

test-rust :
	cargo --version
	cargo test

clean-rust :
	cargo clean

## Python ##############################

PYTHON = python3

MODULES = tm generate test *.py

PYLINT = $(PYTHON) -m pylint

lint : clippy rust
	$(PYLINT) --version
	$(PYLINT) --enable-all-extensions $(MODULES)
	$(MAKE) type

MYPY = $(PYTHON) -m mypy

type :
	$(MYPY) --version
	$(MYPY) $(MODULES)

MYPYC = $(PYTHON) -m mypyc

compile : rust
	$(MYPYC) --version
	$(MYPYC) tm generate test/utils.py --exclude rust_stuff

TUR = test.test_turing.Fast
PROG = test.test_program
GRAPH = test.test_graph
TREEF = test.test_tree.Fast
COV = test.test_coverage
LR = test.test_lin_rado
CG = test.test_code
TP = test.test_tape
NUM = test.test_num
RUL = test.test_rules

SHORT_TESTS = $(PROG) $(GRAPH) $(CG) $(RUL) $(TP) $(COV) $(NUM)

PYTEST = $(PYTHON) -m unittest

test : rust
	$(PYTEST) -v $(SHORT_TESTS) $(LR) $(TREEF) $(TUR)

test-all : test-rust compile
	$(PYTEST) discover -v

COVERAGE = $(PYTHON) -m coverage

coverage : rust
	$(COVERAGE) --version
	$(COVERAGE) run -m unittest -v $(SHORT_TESTS)

	$(COVERAGE) combine --quiet
	$(COVERAGE) html

diagrams :
	pyreverse --only-classnames --colorized -o png tm generate test

# PYTHONPATH=$PYTHONPATH:tm make special target=tm/tape.py
special :
	specialist --target $(target) -m unittest $(TUR)

mutmut :
	-mutmut run
	mutmut html

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
