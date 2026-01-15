.PHONY : clean test

C_FILES := $(wildcard *.c)
RUN_LIN := ${C_FILES:.c=}

CFLAGS := -O3 -std=c11 -Wall -Wextra

test : clean $(RUN_LIN)

% : %.c
	$(CC) $(CFLAGS) $< -o $@.run
	./$@.run | tee /dev/tty | diff -u expected -

clean :
	$(RM) ${C_FILES:.c=.run}
