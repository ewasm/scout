all: build test

build:
	cargo build --release
	chisel run --config chisel.toml; true

test:
	node relayer/index.js
	../../target/release/phase2-scout smpt.yaml
