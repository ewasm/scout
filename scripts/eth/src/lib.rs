#[macro_use]
extern crate ssz_derive;

use ewasm_api::*;
use sha3::{Digest, Keccak256};
use ssz::{Decode, Encode};

#[derive(Debug, PartialEq, Ssz, Default)]
struct Transaction {
    pub target: u64,
    pub data: Vec<u8>,
}

#[derive(Debug, PartialEq, Ssz, Default)]
struct Storage {
    pub code: Vec<u8>,
    // pub storage: Vec<u8>,
}

#[derive(Debug, PartialEq, Ssz, Default)]
struct State {
    pub storage: Vec<Storage>,
}

#[derive(Debug, PartialEq, Ssz, Default)]
struct InputBlock {
    pub transactions: Vec<Transaction>,
    pub state: State,
}

trait StateRoot {
    fn state_root(&self) -> types::Bytes32;
}

impl StateRoot for State {
    fn state_root(&self) -> types::Bytes32 {
        let serialised = self.encode();
        let hash = Keccak256::digest(&serialised[..]);
        let mut ret = types::Bytes32::default();
        ret.bytes.copy_from_slice(&hash[..]);
        ret
    }
}

fn process_block(pre_state: types::Bytes32, mut block_data: &[u8]) -> types::Bytes32 {
    let mut block = InputBlock::decode(&mut block_data).expect("valid input");

    // Validate pre state
    // FIXME: add PartialEq on Bytes32
    assert!(block.state.state_root().bytes == pre_state.bytes);

    for tx in block.transactions {
        // if the target is 0, initialize a new contract
        if tx.target == 0 {
            // skip over 0 if this is the first contract initialization
            let contract_count = block.state.storage.len();
            if contract_count == 0 {
                block.state.storage.push(Storage { code: vec![] });
            }

            block.state.storage.push(Storage { code: tx.data });
        } else {
            eth2::exec_code(&block.state.storage[tx.target as usize].code);
        }
    }

    block.state.state_root()
}

#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main() {
    let pre_state = eth2::load_pre_state();
    let block_data = eth2::acquire_block_data();
    let post_state = process_block(pre_state, &block_data);
    eth2::save_post_state(post_state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_block() {
        let block = InputBlock::default();

        // Lets say the previous state was empty
        let pre_state = block.state.state_root();

        // Process the input block, we're adding nothing to it
        let post_state = process_block(pre_state, &block.encode());

        assert!(
            post_state.bytes
                == [
                    34, 234, 155, 4, 95, 135, 146, 23, 11, 69, 236, 98, 156, 152, 225, 185, 43,
                    198, 161, 156, 216, 208, 233, 243, 123, 170, 173, 242, 86, 65, 66, 244
                ]
        );

        assert!(pre_state.bytes == post_state.bytes)
    }
}
