# Makefile can be used if Cargo isn't available.

CFLAGS = -g

all: lib example

lib:
	@mkdir -p build
	@rustc --out-dir build src/lib.rs $(CFLAGS)

test:
	@mkdir -p build
	@rustc --out-dir build --test src/lib.rs
	@build/dominion

example:
	@rustc -L build example/main.rs $(CFLAGS)

docs:
	@mkdir -p doc
	@rustdoc -o doc src/lib.rs

clean:
	@rm -rf main main.exe build

.PHONY: lib example all clean
