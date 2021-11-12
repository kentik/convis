CLANG   ?= clang
LLC     ?= llc
OPT     ?= opt
DIS     ?= llvm-dis

ARCH    ?= $(shell uname -m | sed -e 's/aarch64/arm64/' -e 's/x86_64/x86/')
KERNEL  ?= /usr/src/linux

INCLUDES :=                                  \
  -I$(KERNEL)/include                        \
  -I$(KERNEL)/arch/$(ARCH)/include           \
  -I$(KERNEL)/arch/$(ARCH)/include/generated \
  -I$(KERNEL)/arch/$(ARCH)/include/uapi      \
  -I$(KERNEL)/include/uapi                   \
  -I$(KERNEL)/include/generated/uapi         \
  -I$(KERNEL)/tools/testing/selftests/bpf

CFLAGS += \
	-O2 -g -emit-llvm                        \
	-D__KERNEL__                             \
	-D__BPF_TRACING__                        \
	-Wno-unused-value                        \
	-Wno-pointer-sign                        \
	-Wno-compare-distinct-pointer-types      \
	-Wno-address-of-packed-member            \
	-Wno-tautological-compare                \
	-Wno-unknown-warning-option              \
	-Wno-gnu-variable-sized-type-not-at-end  \
	-fno-asynchronous-unwind-tables

bytecode.$(ARCH).o: bytecode.c
	$(CLANG) $(CFLAGS) $(INCLUDES) -c $< -o -  | \
	$(OPT) -O2 -mtriple=bpf-pc-linux           | \
	$(DIS)                                     | \
	$(LLC) -march=bpf $(LLC_FLAGS) -filetype=obj -o $@
