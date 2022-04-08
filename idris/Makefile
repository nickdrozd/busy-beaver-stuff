.PHONY : all clean run test version

all : clean version run test

clean :
	rm -rf build/

IDRIS = idris2

version :
	$(IDRIS) --version

% : %.idr
	$(IDRIS) $< -o $@

MACHINES = ../machines

BUILD = build/exec

run : Run
	$(BUILD)/Run 2 2 < $(MACHINES)/2-2.test
	$(BUILD)/Run 3 2 < $(MACHINES)/3-2.test
	$(BUILD)/Run 2 3 < $(MACHINES)/2-3.test
	$(BUILD)/Run 2 4 < $(MACHINES)/2-4.test
	$(BUILD)/Run 4 2 < $(MACHINES)/4-2.test
	$(BUILD)/Run 5 2 < $(MACHINES)/5-2.test

test : Test
	time -p $(BUILD)/Test

test-all : Test
	time -p $(BUILD)/Test --slow-fake-arg
