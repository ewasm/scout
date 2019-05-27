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

Need Rust first. Then install `chisel` using cargo:
```sh
cargo install chisel
```

There is a `Makefile` to make building easy:
- `build` will build all components (the runner and the example scripts)
- `test` will run tests using the YAML test files
- `all` will do both

The runner is called scout and is available at `target/release/phase2-scout` after being built.

The runner expects a YAML test file:
```yaml
beacon_state:
  execution_scripts:
    - scripts/helloworld/target/wasm32-unknown-unknown/release/phase2_helloworld.wasm
shard_pre_state:
  exec_env_states:
    - "0000000000000000000000000000000000000000000000000000000000000000"
shard_blocks:
  - env: 0
    data: ""
  - env: 0
    data: ""
shard_post_state:
  exec_env_states:
    - "0000000000000000000000000000000000000000000000000000000000000000"
```

The runner expects a filename pointing to the test file or will default to `test.yaml` in the local directory if nothing was specified.

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

A better example is located in `scripts/bazaar` which is in essence a stateless contract. It uses SSZ serialisation. A test case is included in `bazaar.yaml`.

It should be possible to import any Rust crate as long as it can be compiled to the wasm32 target.

## Maintainer

* Alex Beregszaszi

## License

Apache 2.0
