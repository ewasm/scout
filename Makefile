all: build test

build:
	cd scripts/helloworld && cargo build --release && chisel run --config chisel.toml
	cd scripts/bazaar && cargo build --release && chisel run --config chisel.toml
	cd scripts/snark-verifier && cargo build --release && chisel run --config chisel.toml
	cargo build --release

test:
	target/release/phase2-scout
	target/release/phase2-scout bazaar.yaml
	target/release/phase2-scout snark-verifier.yaml
