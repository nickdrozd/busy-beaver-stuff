all :
	idris2 Main.idr -o prog
	time -p ./build/exec/prog

clean :
	rm -rf build/
