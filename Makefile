.PHONY : all run lint test time profile clean machines

all : lint test

run :
	python3 turing.py

lint :
	pylint *.py **/*.py

test :
	python3 -m unittest test.py

time : test

profile :
	python3 -m cProfile turing.py

clean :
	rm -rf yappi.* __pycache__ **/run

machines :
	$(MAKE) -C 3-2
	$(MAKE) -C 2-3
	$(MAKE) -C 4-2
	$(MAKE) -C 2-4
	$(MAKE) -C 5-2
