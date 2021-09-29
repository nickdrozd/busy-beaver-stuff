.PHONY : test clean

test : clean
	idris2 --version
	idris2 Test.idr -o test
	time -p ./build/exec/test

clean :
	rm -rf build/
