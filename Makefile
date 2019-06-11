all: build test

build:
	cd scripts/helloworld && cargo build --release && chisel run --config chisel.toml
	cd scripts/bazaar && cargo build --release && chisel run --config chisel.toml
	cd scripts/executor && cargo build --release && chisel run --config chisel.toml
	cd scripts/executor/contracts/kvstore && cargo build --release && chisel run --config chisel.toml && wasm-snip target/wasm32-unknown-unknown/release/kvstore.wasm -o target/wasm32-unknown-unknown/release/kvstore.wasm --snip-rust-fmt-code --snip-rust-panicking-code
	cargo build --release

test:
	target/release/phase2-scout executor.yaml
