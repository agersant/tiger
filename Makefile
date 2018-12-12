TIGER_BIN_DIR := ~/.local/bin/tiger

all: build

build:
	cargo build --release

install: build
	install -d $(TIGER_BIN_DIR)
	install ./target/release/tiger $(TIGER_BIN_DIR)
	@echo "Tiger installation complete!"

clean:
	cargo clean

uninstall:
	rm -r $(TIGER_BIN_DIR)
