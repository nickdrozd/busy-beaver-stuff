C_FILES := $(wildcard *.c)
MACHINES := ${C_FILES:.c=}

.PHONY : test clean

CFLAGS += -Wall -Wextra -O3

test : clean $(MACHINES)
	$(MAKE) -C structured

% : %.c
	$(CC) $(CFLAGS) $< -o $@.run
	sed 's/^1RB// ; s/ //g' $@.test | ./$@.run

clean :
	$(RM) ${C_FILES:.c=.run}
	$(MAKE) -C structured clean
