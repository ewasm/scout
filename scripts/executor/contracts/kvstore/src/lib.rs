#[macro_use]
extern crate ssz_derive;

use ewasm_api::*;
use ssz::{Decode, Encode};

#[derive(Clone, Debug, PartialEq, Ssz, Default)]
struct Storage {
    pub code: Vec<u8>,
    pub storage: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Ssz, Default)]
struct State {
    pub storage: Vec<Storage>,
}

#[derive(Debug, PartialEq, Ssz, Default)]
struct Context {
    pub state: State,
}

#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main() {
    let mut ctx: &[u8] = &eth2::acquire_context_data()[..];
    let mut ctx = Context::decode(&mut ctx).expect("valid context");
    let calldata = eth2::acquire_block_data();
    ctx.state.storage[1].storage.push(calldata[0]);
    eth2::save_return_data(&ctx.encode());
}
