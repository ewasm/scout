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
            // eth2::exec_code(&block.state.storage[tx.target as usize].code);
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
    #[ignore]
    fn create_inputs() {
        let mut block = InputBlock::default();
        let tx = Transaction {
            target: 0,
            // adder.wasm
            data: vec![
                0, 97, 115, 109, 1, 0, 0, 0, 1, 132, 128, 128, 128, 0, 1, 96, 0, 0, 3, 130, 128,
                128, 128, 0, 1, 0, 5, 131, 128, 128, 128, 0, 1, 0, 16, 7, 145, 128, 128, 128, 0, 2,
                6, 109, 101, 109, 111, 114, 121, 2, 0, 4, 109, 97, 105, 110, 0, 0, 10, 132, 128,
                128, 128, 0, 1, 2, 0, 11, 0, 146, 128, 128, 128, 0, 4, 110, 97, 109, 101, 1, 135,
                128, 128, 128, 0, 1, 0, 4, 109, 97, 105, 110, 0, 222, 128, 128, 128, 0, 9, 112,
                114, 111, 100, 117, 99, 101, 114, 115, 2, 8, 108, 97, 110, 103, 117, 97, 103, 101,
                1, 4, 82, 117, 115, 116, 4, 50, 48, 49, 56, 12, 112, 114, 111, 99, 101, 115, 115,
                101, 100, 45, 98, 121, 2, 5, 114, 117, 115, 116, 99, 29, 49, 46, 51, 52, 46, 48,
                32, 40, 57, 49, 56, 53, 54, 101, 100, 53, 50, 32, 50, 48, 49, 57, 45, 48, 52, 45,
                49, 48, 41, 6, 119, 97, 108, 114, 117, 115, 5, 48, 46, 52, 46, 48,
            ],
        };

        block.transactions.push(tx);

        println!("{:?}", block.encode());
        println!("{}", hex::encode(block.encode()));
    }

    #[test]
    fn empty_block() {
        let block = InputBlock::default();
        let pre_state = block.state.state_root();
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

    #[test]
    fn store_multiple_contracts() {
        let mut block = InputBlock::default();

        let contract = vec![
            0, 97, 115, 109, 1, 0, 0, 0, 1, 132, 128, 128, 128, 0, 1, 96, 0, 0, 3, 130, 128, 128,
            128, 0, 1, 0, 5, 131, 128, 128, 128, 0, 1, 0, 16, 7, 145, 128, 128, 128, 0, 2, 6, 109,
            101, 109, 111, 114, 121, 2, 0, 4, 109, 97, 105, 110, 0, 0, 10, 132, 128, 128, 128, 0,
            1, 2, 0, 11, 0, 146, 128, 128, 128, 0, 4, 110, 97, 109, 101, 1, 135, 128, 128, 128, 0,
            1, 0, 4, 109, 97, 105, 110, 0, 222, 128, 128, 128, 0, 9, 112, 114, 111, 100, 117, 99,
            101, 114, 115, 2, 8, 108, 97, 110, 103, 117, 97, 103, 101, 1, 4, 82, 117, 115, 116, 4,
            50, 48, 49, 56, 12, 112, 114, 111, 99, 101, 115, 115, 101, 100, 45, 98, 121, 2, 5, 114,
            117, 115, 116, 99, 29, 49, 46, 51, 52, 46, 48, 32, 40, 57, 49, 56, 53, 54, 101, 100,
            53, 50, 32, 50, 48, 49, 57, 45, 48, 52, 45, 49, 48, 41, 6, 119, 97, 108, 114, 117, 115,
            5, 48, 46, 52, 46, 48,
        ];

        let tx = Transaction {
            target: 0,
            data: contract.clone(),
        };

        block.transactions.push(tx);

        let pre_state = block.state.state_root();
        let post_state = process_block(pre_state, &block.encode());

        let result = State {
            storage: vec![
                Storage {
                    code: contract.clone(),
                },
                Storage {
                    code: contract.clone(),
                },
            ],
        };

        println!("{:?}", post_state.bytes);
        println!("{:?}", result.state_root().bytes);
        assert!(post_state.bytes == result.state_root().bytes);
    }
}
