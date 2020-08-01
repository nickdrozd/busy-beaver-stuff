lint :
	pylint turing.py test.py

test :
	python3 -m unittest test.py

profile :
	python3 turing.py --profile yes
