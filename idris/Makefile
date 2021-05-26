all : clean
	idris2 --version
	idris2 Main.idr -o prog
	time -p ./build/exec/prog

clean :
	rm -rf build/
