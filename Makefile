build:
	cd scripts/helloworld && cargo build --release
	cd scripts/bazaar && cargo build --release
	cargo build

test:
	target/debug/phase2-scout
	target/debug/phase2-scout bazaar.yaml

all: build test
