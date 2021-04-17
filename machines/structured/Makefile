C_FILES := $(wildcard *.c)
MACHINES := ${C_FILES:.c=}

.PHONY : clean

CFLAGS += -Wall -Wextra -O3

test : $(MACHINES)

% : %.c
	$(CC) $(CFLAGS) $< -o $@.run -D XLIMIT=$@
	./$@.run

clean :
	$(RM) ${C_FILES:.c=.run}
