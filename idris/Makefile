.PHONY : all clean run test

all : test

clean :
	rm -rf build/

run : clean
	idris2 --version
	idris2 Run.idr -o run
	echo "1RB 2RA 1RA 2RB  2LB 3LA 0RB 2LA\n1RB 2LB 3RA 2LA  3LB 3RA 0RB 1RB" | ./build/exec/run 2 4

test : clean
	idris2 --version
	idris2 Test.idr -o test
	time -p ./build/exec/test
