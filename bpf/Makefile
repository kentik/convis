CLANG   ?= clang
LLC     ?= llc
KERNEL  ?= /usr/src/linux

INCLUDES :=                                  \
  -I$(KERNEL)/include                        \
  -I$(KERNEL)/arch/x86/include               \
  -I$(KERNEL)/arch/x86/include/generated     \
  -I$(KERNEL)/arch/x86/include/uapi          \
  -I$(KERNEL)/include/uapi                   \
  -I$(KERNEL)/include/generated/uapi         \
  -I$(KERNEL)/tools/testing/selftests/bpf

CFLAGS += \
	-D__KERNEL__                             \
	-D__ASM_SYSREG_H                         \
	-D__BPF_TRACING__                        \
	-Wno-unused-value                        \
	-Wno-pointer-sign                        \
	-Wno-compare-distinct-pointer-types      \
	-Wno-address-of-packed-member            \
	-Wno-tautological-compare                \
	-Wno-unknown-warning-option              \
	-Wno-gnu-variable-sized-type-not-at-end  \
	-fno-asynchronous-unwind-tables

bytecode.o: bytecode.c
	$(CLANG) $(CFLAGS) $(INCLUDES) -O2 -emit-llvm -c $< -o - | $(LLC) -march=bpf -filetype=obj -o $@
