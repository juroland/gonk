include .env
export

BIN ?= main

.PHONY: build run flash clean check help

help:
	@echo "Available targets:"
	@echo "  make build              - Build the project"
	@echo "  make run BIN=<name>     - Run a binary (default: main)"
	@echo "  make flash BIN=<name>   - Flash a binary to device (default: main)"
	@echo "  make check              - Check the project"
	@echo "  make clean              - Clean build artifacts"
	@echo ""
	@echo "Example: make flash BIN=test_wifi"

build:
	cargo build

run:
	cargo run --bin $(BIN)

flash:
	cargo run --bin $(BIN) --release

clean:
	cargo clean

check:
	cargo check