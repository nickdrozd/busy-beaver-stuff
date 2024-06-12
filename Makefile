.PHONY : all clean clean-python compile coverage generate idris lint machines profile special test test-all type

all : machines idris lint test tools

clean-python :
	rm -rf __pycache__ **/__pycache__ .mypy_cache .mutmut-cache .ruff_cache .coverage* html* build/ *.so **/*.so classes.png packages.png

clean : clean-python clean-rust
	$(MAKE) -C machines clean
	$(MAKE) -C idris clean

refresh : clean-python rust

quick-check : refresh test-rust lint coverage

## Odd langs ###########################

machines :
	$(MAKE) -C machines

idris :
	$(MAKE) -C idris

## Rust ################################

RUST_STUFF = tm/rust_stuff.so

rust :
	cargo build --release
	cp target/release/librust_stuff.so $(RUST_STUFF)

clippy :
	cargo --version
	cargo clippy

test-rust :
	cargo --version
	cargo test

test-rust-all :
	cargo --version
	cargo test -- --include-ignored

clean-rust :
	cargo clean

## Python ##############################

PYTHON = python3

MODULES = tm tools test perf *.py

RUFF = $(PYTHON) -m ruff
PYLINT = $(PYTHON) -m pylint

lint : clippy rust
	$(RUFF) --version
	$(RUFF) check $(MODULES)
	$(MAKE) type
	$(PYLINT) --version
	$(PYLINT) --enable-all-extensions $(MODULES) --ignore-patterns=.*.pyi

MYPY = $(PYTHON) -m mypy

type :
	$(MYPY) --version
	$(MYPY) $(MODULES)

MYPYC = $(PYTHON) -m mypyc

TEST_COMPILE = test/utils.py test/lin_rec.py test/machine.py

compile : rust
	$(MYPYC) --version
	$(MYPYC) tm tools $(TEST_COMPILE) --exclude rust_stuff

TUR = test.test_turing
PROG = test.test_program
GRAPH = test.test_graph
COV = test.test_coverage
CG = test.test_code
TP = test.test_tape
NUM = test.test_num
RUL = test.test_rules

SHORT_TESTS = $(PROG) $(GRAPH) $(CG) $(RUL) $(TP) $(COV) $(NUM)

PYTEST = $(PYTHON) -m unittest

test : test-rust
	$(PYTEST) discover -v

test-all : test-rust-all compile
	RUN_SLOW=1 $(MAKE) test

tree : compile
	RUN_SLOW=1 $(PYTEST) test.test_tree
	$(MAKE) refresh

COVERAGE = $(PYTHON) -m coverage

coverage : rust
	$(COVERAGE) --version
	$(COVERAGE) run -m unittest -v $(SHORT_TESTS)

	$(COVERAGE) combine --quiet
	$(COVERAGE) html

diagrams :
	pyreverse --only-classnames --no-standalone --colorized -o png tm tools test perf

# PYTHONPATH=$PYTHONPATH:tm make special target=tm/tape.py
special :
	specialist --target $(target) -m unittest $(TUR)

mutmut :
	-mutmut run
	mutmut html

profile :
	$(MAKE) -C perf

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
