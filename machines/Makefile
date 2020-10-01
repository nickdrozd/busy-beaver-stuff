C_FILES := $(wildcard *.c)
MACHINES := ${C_FILES:.c=}

.PHONY : test clean

CFLAGS += -Wall -Wextra -O3

test : $(MACHINES)

% : %.c
	$(CC) $(CFLAGS) $< -o $@.run
	./$@.run < $@.test

clean :
	$(RM) $(MACHINES)
