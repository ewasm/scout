all: build test

build:
	cd scripts/helloworld && cargo build --release && chisel run --config chisel.toml
	cd scripts/bazaar && cargo build --release && chisel run --config chisel.toml
	cargo build

test:
	target/debug/phase2-scout
	target/debug/phase2-scout bazaar.yaml
