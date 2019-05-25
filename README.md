# Scout

Scout is a Ethereum 2.0 Phase 2 execution prototyping engine.

**Warning: this is super experimental**

## Goals

1) Create a (black boxed) execution prototyping engine
2) Create some example contracts ("execution scripts")
3) Enable "easy" onboarding for creating scripts
4) By having actual real world use cases in scripts, we can benchmark the design and identify bottlenecks

## What is this?

This engine intentionally avoids a lot of details and therefore it is not usable as a Eth 2.0 client.
Instead of being a client, it should support reading and outputting shard/beacon states in a YAML format.

## How to use this?

Need Rust first. The build system isn't too well integrated at the moment.

Build the example script first:
```sh
$ cd scripts/helloworld
$ cargo build --release
```

Build the runner second:
```sh
cargo build
```

The runner expects a `phase2_helloworld.wasm` file to be in the same directory. It will print pre and post states. The pre state is pretty much empty, but has two copies of the execution script.

## How to code scripts?

An example script is located in `scripts/helloworld`. It uses a branch of [ewasm-rust-api](https://github.com/ewasm/ewasm-rust-api/tree/eth2-phase2).

```rust
extern crate ewasm_api;

use ewasm_api::*;

#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main() {
    let pre_state = eth2::load_pre_state();

    assert!(eth2::block_data_size() == 0);

    // No updates were made to the state
    let post_state = pre_state;

    eth2::save_post_state(post_state)
}
```

It should be possible to import any Rust crate as long as it can be compiled to the wasm32 target.

## Maintainer

* Alex Beregszaszi

## License

Apache 2.0
