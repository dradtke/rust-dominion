all:: lib example

lib:
	@echo "Building libdominion..."
	@mkdir -p build
	@rustc -g --out-dir build dominion/mod.rs $(CFLAGS)

test:
	@echo "Building libdominion tests..."
	@mkdir -p build
	@rustc --out-dir build --test dominion/mod.rs
	@echo "Running..."
	@build/dominion

example:
	@echo "Building main..."
	@rustc -g -L build main.rs $(CFLAGS)

clean:
	@echo "Cleaning..."; rm -rf main main.exe build

.PHONY: lib example all clean
