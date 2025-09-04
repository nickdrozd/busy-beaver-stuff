.PHONY : all clean clean-python compile coverage idris lint machines rust test test-all type

all : machines idris lint test tools

clean-python :
	rm -rf __pycache__ **/__pycache__ .mypy_cache .ruff_cache .coverage* html* build/ *.so **/*.so classes.png packages.png mypyc_annotation.html

clean : clean-python clean-rust
	$(MAKE) -C machines clean
	$(MAKE) -C idris clean

refresh : clean-python rust

quick-check : refresh trust lint coverage

## Odd langs ###########################

machines :
	$(MAKE) -C machines

idris :
	$(MAKE) -C idris

## Rust ################################

CARGO = cargo

RUSTFLAGS = RUSTFLAGS="-C target-cpu=native"

BUILD = $(RUSTFLAGS) $(CARGO) build
REL_TARGET = target/release/libexport.so
DEV_TARGET = target/debug/libexport.so

PYEXTENSIONS = tm/rust_stuff.so

MOVE_BUILD = cp $(BUILD_TARGET) $(PYEXTENSIONS)

rust :
	$(BUILD) --release --package export
	cp $(REL_TARGET) $(PYEXTENSIONS)

dev :
	$(BUILD) --package export
	cp $(DEV_TARGET) $(PYEXTENSIONS)

CARGO_VERSION = cargo --version

clippy :
	$(CARGO_VERSION)
	$(CARGO) fmt --check
	$(CARGO) clippy --all-targets

CARGO_TEST = $(CARGO) test

trust :
	$(CARGO_VERSION)
	$(CARGO_TEST)

run :
	$(CARGO_VERSION)
	$(BUILD) --release --package run
	$(RUSTFLAGS) time -p cargo run --release -p run

run-all:
	$(CARGO_VERSION)
	$(RUSTFLAGS) $(CARGO) run --release -p run -- --all

clean-rust :
	cargo clean

## Python ##############################

PYTHON = python3

MODULES = tm tools test *.py

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

compile : rust
	$(MYPYC) --version
	$(MYPYC) tm tools test/lin_rec.py --exclude rust_stuff

mypyc-report : rust
	$(MYPYC) --version
	$(MYPYC) tm tools --exclude rust_stuff -a mypyc-report.html

PYTEST = $(PYTHON) -m unittest

test :
	$(PYTEST) discover -v

test-all : compile
	RUN_SLOW=1 $(MAKE) test

COVERAGE = $(PYTHON) -m coverage

COV_TESTS = test.test_coverage test.test_program test.test_num

coverage : rust
	$(COVERAGE) --version
	$(COVERAGE) run -m unittest -v $(COV_TESTS)

	$(COVERAGE) combine --quiet
	$(COVERAGE) html

diagrams :
	pyreverse --only-classnames --no-standalone --colorized -o png tm tools test
