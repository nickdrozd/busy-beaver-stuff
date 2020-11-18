C_FILES := $(wildcard *.c)
MACHINES := ${C_FILES:.c=}

.PHONY : test clean

CFLAGS += -Wall -Wextra -O3

test : $(MACHINES)

% : %.c
	$(CC) $(CFLAGS) $< machine.h -o $@.run
	./$@.run < $@.test

clean :
	$(RM) ${C_FILES:.c=.run}
