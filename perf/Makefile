.PHONY : all clean

PY := $(filter-out __init__.py, $(wildcard *.py))
CG := $(PY:.py=.callgrind)
PNG := $(PY:.py=.png)

all: $(PNG)

clean :
	rm -f ${CG}

%.callgrind: %.py
	cd .. && python3 -m perf.$*

%.png: %.callgrind
	gprof2dot $< -f callgrind --colour-nodes-by-selftime | dot -T png -o $@
