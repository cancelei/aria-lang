# Aria Runtime Library Makefile
#
# This Makefile builds the Aria runtime library and provides targets
# for linking Aria programs into complete executables.

# Compiler and flags
CC = gcc
CFLAGS = -Wall -Wextra -O2 -std=c11
LDFLAGS =

# Runtime library
RUNTIME_SRC = aria_runtime.c
RUNTIME_OBJ = aria_runtime.o
RUNTIME_HEADER = aria_runtime.h

# Default target: build the runtime library
.PHONY: all
all: $(RUNTIME_OBJ)

# Build the runtime object file
$(RUNTIME_OBJ): $(RUNTIME_SRC) $(RUNTIME_HEADER)
	$(CC) $(CFLAGS) -c $(RUNTIME_SRC) -o $(RUNTIME_OBJ)

# Link an Aria program with the runtime
# Usage: make link PROGRAM=myprogram.o OUTPUT=myprogram
.PHONY: link
link: $(RUNTIME_OBJ)
ifndef PROGRAM
	@echo "Error: PROGRAM variable not set"
	@echo "Usage: make link PROGRAM=myprogram.o OUTPUT=myprogram"
	@exit 1
endif
ifndef OUTPUT
	@echo "Error: OUTPUT variable not set"
	@echo "Usage: make link PROGRAM=myprogram.o OUTPUT=myprogram"
	@exit 1
endif
	$(CC) $(LDFLAGS) $(RUNTIME_OBJ) $(PROGRAM) -o $(OUTPUT)
	@echo "Created executable: $(OUTPUT)"

# Clean build artifacts
.PHONY: clean
clean:
	rm -f $(RUNTIME_OBJ)

# Install the runtime library (copy to a standard location)
# Usage: make install PREFIX=/usr/local
.PHONY: install
install: $(RUNTIME_OBJ)
	@PREFIX=$${PREFIX:-/usr/local}; \
	LIBDIR="$$PREFIX/lib/aria"; \
	INCDIR="$$PREFIX/include/aria"; \
	echo "Installing to $$PREFIX..."; \
	mkdir -p "$$LIBDIR" "$$INCDIR"; \
	cp $(RUNTIME_OBJ) "$$LIBDIR/"; \
	cp $(RUNTIME_HEADER) "$$INCDIR/"; \
	echo "Installed runtime library to $$LIBDIR"; \
	echo "Installed headers to $$INCDIR"

# Show help
.PHONY: help
help:
	@echo "Aria Runtime Library Makefile"
	@echo ""
	@echo "Targets:"
	@echo "  all                    Build the runtime library (default)"
	@echo "  link                   Link an Aria program with the runtime"
	@echo "                         Usage: make link PROGRAM=prog.o OUTPUT=prog"
	@echo "  clean                  Remove build artifacts"
	@echo "  install                Install runtime library to PREFIX (default: /usr/local)"
	@echo "  help                   Show this help message"
	@echo ""
	@echo "Examples:"
	@echo "  make                                          # Build runtime"
	@echo "  make link PROGRAM=hello.o OUTPUT=hello        # Link program"
	@echo "  make install PREFIX=/opt/aria                 # Install to /opt/aria"
