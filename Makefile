.PHONY : all run lint test time profile clean machines

all : machines structured lint test

run :
	python3 run.py

lint :
	pylint --version
	pylint *.py **/*.py

test :
	python3 -m unittest discover -v test

time : test

profile :
	python3 -m cProfile turing.py

clean :
	rm -rf yappi.* __pycache__ **/__pycache__ **/run
	$(MAKE) -C machines clean

machines :
	$(MAKE) -C machines

structured :
	$(MAKE) -C machines/structured
