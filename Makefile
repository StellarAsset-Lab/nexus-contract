.PHONY: build test fmt fmt-check clippy check clean

build:
	stellar contract build

test:
	cargo test --workspace

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

clippy:
	cargo clippy --workspace --all-targets -- -D warnings

check: fmt-check clippy test

clean:
	cargo clean
