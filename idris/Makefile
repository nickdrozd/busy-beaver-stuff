.PHONY : all clean run test

all : run test

clean :
	rm -rf build/

run : clean
	idris2 --version
	idris2 Run.idr -o run
	./build/exec/run 2 2 < ../machines/2-2.test
	./build/exec/run 3 2 < ../machines/3-2.test
	./build/exec/run 2 3 < ../machines/2-3.test
	./build/exec/run 2 4 < ../machines/2-4.test
	./build/exec/run 4 2 < ../machines/4-2.test
	./build/exec/run 5 2 < ../machines/5-2.test

test : clean
	idris2 --version
	idris2 Test.idr -o test
	time -p ./build/exec/test
