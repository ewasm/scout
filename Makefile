all: build test

build:
	cd scripts/helloworld && cargo build --release && chisel run --config chisel.toml
	cd scripts/bazaar && cargo build --release && chisel run --config chisel.toml
	cd scripts/eth && cargo build --release && chisel run --config chisel.toml
	cd scripts/eth/contracts/adder && cargo build --release && chisel run --config chisel.toml && wasm-snip target/wasm32-unknown-unknown/release/adder.wasm -o target/wasm32-unknown-unknown/release/adder.wasm --snip-rust-fmt-code --snip-rust-panicking-code
	cargo build --release

test:
	target/release/phase2-scout
	target/release/phase2-scout bazaar.yaml
