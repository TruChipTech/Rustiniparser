# Build the Rust libraries, then compile and run the C/C++ FFI samples & tests.
#
#   make            # build everything
#   make test       # build + run the C and C++ assertion tests
#   make run        # build + run the C and C++ samples
#   make clean      # remove compiled C/C++ artifacts (not the Rust target/)
#
# Static linking is used by default (no LD_LIBRARY_PATH needed). Set
# LINK=shared to link against librustiniparser.so instead.

ROOT    := ../..
INCLUDE := $(ROOT)/include
RELEASE := $(ROOT)/target/release
STATIC  := $(RELEASE)/librustiniparser.a

CC      ?= cc
CXX     ?= c++
CFLAGS  := -Wall -Wextra -I$(INCLUDE)
CXXFLAGS:= -Wall -Wextra -std=c++11 -I$(INCLUDE)

# System libraries the Rust staticlib needs when linked into a C/C++ program.
SYSLIBS := -lpthread -ldl -lm

LINK ?= static
ifeq ($(LINK),shared)
  LINKFLAGS := -L$(RELEASE) -lrustiniparser
  RUN       := LD_LIBRARY_PATH=$(RELEASE)
else
  LINKFLAGS := $(STATIC) $(SYSLIBS)
  RUN       :=
endif

BIN := sample_c sample_cpp test_c test_cpp

.PHONY: all libs test run clean

all: $(BIN)

# Build the Rust .a/.so artifacts.
libs:
	cd $(ROOT) && cargo build --release

sample_c: $(ROOT)/examples/c/sample.c | libs
	$(CC) $(CFLAGS) $< $(LINKFLAGS) -o $@

sample_cpp: $(ROOT)/examples/cpp/sample.cpp | libs
	$(CXX) $(CXXFLAGS) $< $(LINKFLAGS) -o $@

test_c: test_ffi.c | libs
	$(CC) $(CFLAGS) $< $(LINKFLAGS) -o $@

test_cpp: test_ffi.cpp | libs
	$(CXX) $(CXXFLAGS) $< $(LINKFLAGS) -o $@

test: test_c test_cpp
	@echo "== C test =="   && $(RUN) ./test_c
	@echo "== C++ test ==" && $(RUN) ./test_cpp

run: sample_c sample_cpp
	@echo "== C sample =="   && $(RUN) ./sample_c
	@echo "== C++ sample ==" && $(RUN) ./sample_cpp

clean:
	rm -f $(BIN)
