
all:: lib example

lib:
	@echo "Building libdominion..."
	@mkdir -p build
	@rustc --out-dir build dominion/mod.rs

example:
	@echo "Building main..."
	@rustc -L build main.rs

clean:
	@echo "Cleaning..."; rm -rf main build

.PHONY: lib example all clean
