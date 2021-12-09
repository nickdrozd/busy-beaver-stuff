.PHONY : all clean run test

all : test

clean :
	rm -rf build/

run : clean
	idris2 --version
	idris2 Run.idr -o run
	echo "1RB 2RA 1RA 2RB  2LB 3LA 0RB 2LA" | ./build/exec/run
	time -p ./build/exec/run < ../2-4-with-blank.prog

test : clean
	idris2 --version
	idris2 Test.idr -o test
	time -p ./build/exec/test
