all:: lib example

CFLAGS = -g

lib:
	@echo -n "Building libdominion..."
	@mkdir -p build
	@rustc --out-dir build dominion/mod.rs $(CFLAGS)
	@echo "done."

test:
	@echo -n "Building libdominion tests..."
	@mkdir -p build
	@rustc --out-dir build --test dominion/mod.rs
	@echo "done."
	@echo -n "Running..."
	@build/dominion
	@echo "done."

example:
	@echo -n "Building main..."
	@rustc -L build main.rs $(CFLAGS)
	@echo "done."

clean:
	@echo -n "Cleaning..."
	@rm -rf main main.exe build
	@echo "done."

.PHONY: lib example all clean
