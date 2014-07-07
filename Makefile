
CFLAGS = -g

all: lib example

lib:
	@mkdir -p build
	@rustc --out-dir build src/mod.rs $(CFLAGS)

test:
	@mkdir -p build
	@rustc --out-dir build --test src/mod.rs
	@build/dominion

example:
	@rustc -L build example/main.rs $(CFLAGS)

docs:
	@mkdir -p doc
	@rustdoc -o doc src/mod.rs

clean:
	@rm -rf main main.exe build

.PHONY: lib example all clean
