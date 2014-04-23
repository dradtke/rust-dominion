
all:: lib example

lib:
	@echo "Building libdominion..."
	@mkdir -p build
	@rustc --out-dir build dominion/mod.rs

test:
	@echo "Building libdominion tests..."
	@mkdir -p build
	@rustc --out-dir build --test dominion/mod.rs
	@echo "Running..."
	@build/dominion

example:
	@echo "Building main..."
	@rustc -L build main.rs

clean:
	@echo "Cleaning..."; rm -rf main main.exe build

.PHONY: lib example all clean
