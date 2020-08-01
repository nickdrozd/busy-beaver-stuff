run :
	python3 turing.py

lint :
	pylint turing.py test.py

test :
	python3 -m unittest test.py

profile :
	python3 turing.py --profile yes
	gprof2dot yappi.callgrind -f callgrind --colour-nodes-by-selftime | dot -Tpng -o yappi.png
