.SECONDARY: test/*.s test/*.o test/*.a

UNAME := $(shell uname)

ifeq ($(UNAME), Darwin)
ARCH := macho64
RUST_TARGET := x86_64-apple-darwin
else
ARCH := elf64
RUST_TARGET := x86_64-unknown-linux-gnu
endif

SNEK_FILES := $(filter-out test/wrong_% test/undefined_%, $(wildcard test/*.snek))
RUN_FILES := $(patsubst test/%.snek,test/%.run,$(SNEK_FILES))

all: $(RUN_FILES)

test: $(RUN_FILES)
	@for f in $(RUN_FILES); do \
		echo "Running $$f"; \
		arch -x86_64 ./$$f; \
	done

test/%.s: test/%.snek src/main.rs
	cargo run -- $< $@

test/%.o: test/%.s
	nasm -f $(ARCH) $< -o $@

test/%.a: test/%.o
	ar rcs $@ $<

test/%.run: test/%.a runtime/start.rs
	cp $< test/libour_code.a
	rustc --target $(RUST_TARGET) runtime/start.rs -L test -o $@
	rm -f test/libour_code.a

clean:
	rm -f test/*.s test/*.o test/*.a test/*.run test/libour_code.a