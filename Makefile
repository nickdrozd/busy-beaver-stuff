.PHONY : all clean clean-python compile coverage generate idris lint machines profile rust special test test-all type

all : machines idris lint test tools

clean-python :
	rm -rf __pycache__ **/__pycache__ .mypy_cache .mutmut-cache .ruff_cache .coverage* html* build/ *.so **/*.so classes.png packages.png mypyc_annotation.html

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

CARGO = PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 cargo

BUILD = $(CARGO) build --package export
BUILD_TARGET = target/release/libexport.so

PYEXTENSIONS = tm/rust_stuff.so

MOVE_BUILD = cp $(BUILD_TARGET) $(PYEXTENSIONS)

rust :
	$(BUILD) --release
	$(MOVE_BUILD)

dev :
	$(BUILD)
	$(MOVE_BUILD)

CARGO_VERSION = cargo --version

clippy :
	$(CARGO_VERSION)
	$(CARGO) clippy --all-targets

CARGO_TEST = $(CARGO) test

test-rust :
	$(CARGO_VERSION)
	$(CARGO_TEST)

run :
	$(CARGO_VERSION)
	$(CARGO) run --release -p run

run-all:
	$(CARGO_VERSION)
	$(CARGO) run --release -p run -- --all

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

TEST_COMPILE = test/utils.py test/lin_rec.py

compile : rust
	$(MYPYC) --version
	$(MYPYC) tm tools $(TEST_COMPILE) --exclude rust_stuff

mypyc-report : rust
	$(MYPYC) --version
	$(MYPYC) tm tools --exclude rust_stuff -a mypyc-report.html

TUR = test.test_turing
PROG = test.test_program
GRAPH = test.test_graph
COV = test.test_coverage
CG = test.test_code
TP = test.test_tape
NUM = test.test_num
RUL = test.test_rules
MAC = test.test_macro
HOLD = test.test_holdouts

SHORT_TESTS = $(PROG) $(GRAPH) $(CG) $(RUL) $(MAC) $(TP) $(COV) $(NUM) $(HOLD)

PYTEST = $(PYTHON) -m unittest

test :
	$(PYTEST) discover -v

test-all : test-rust compile
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
