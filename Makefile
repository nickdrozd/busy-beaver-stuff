all : lint test

run :
	python3 turing.py

lint :
	pylint turing.py test.py

test :
	python3 -m unittest test.py

time : test

profile :
	python3 turing.py --profile yes

clean :
	rm -rf yappi.* __pycache__
