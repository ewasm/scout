///
/// This is an example Phase 2 script called "Bazaar".
///
/// The "state" consists of an append-only list of messages. Each transaction
/// has to supply the new list of messages to be appended, as well as the entire
/// current state. The hash of the SSZ encoded state is stored as the "state root".
/// Obviously this has an unsustainable growth, but the main point is to demonstrate
/// how to work with SSZ serialised data.
///
/// It doesn't yet use SSZ merkleization and SSZ partial, that should be an obvious next step.
///
/// Message
/// {
///    "timestamp": uint64,
///    "message": bytes
/// }
///
/// State {
///    "messages": [Message]
/// }
///
/// InputBlock {
///    "new_messages": [Message],
///    "state": State
/// }
///
extern crate ewasm_api;
extern crate sha3;
extern crate ssz;

#[macro_use]
extern crate ssz_derive;

use ewasm_api::*;
use sha3::{Digest, Keccak256};
use ssz::{Decode, Encode};

#[derive(Debug, PartialEq, Ssz, Default)]
struct Message {
    pub timestamp: u64,
    pub message: [u8; 32],
}

#[derive(Debug, PartialEq, Ssz, Default)]
struct State {
    pub messages: Vec<Message>,
}

#[derive(Debug, PartialEq, Ssz, Default)]
struct InputBlock {
    pub new_messages: Vec<Message>,
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

fn process_block(pre_state_root: types::Bytes32, mut block_data: &[u8]) -> types::Bytes32 {
    let mut block = InputBlock::decode(&mut block_data).expect("valid input");

    // Validate pre state
    assert!(block.state.state_root() == pre_state_root);

    for message in block.new_messages {
        block.state.messages.push(message)
    }

    #[cfg(test)]
    println!("{:#?}", block.state);

    block.state.state_root()
}

#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main() {
    let pre_state_root = eth2::load_pre_state_root();
    let block_data = eth2::acquire_block_data();
    let post_state_root = process_block(pre_state_root, &block_data);
    eth2::save_post_state_root(post_state_root)
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
        assert!(pre_state == post_state)
    }

    #[test]
    fn two_messages_in_block() {
        let mut block = InputBlock::default();
        // Lets say the previous state was empty
        let pre_state = block.state.state_root();
        // Add new messages now
        block.new_messages.push(Message {
            timestamp: 1,
            message: [0u8; 32],
        });
        block.new_messages.push(Message {
            timestamp: 2,
            message: [1u8; 32],
        });
        // Process the input block, we're adding nothing to it
        let post_state = process_block(pre_state, &block.encode());
        assert!(
            post_state.bytes
                == [
                    41, 80, 95, 217, 82, 133, 123, 87, 102, 199, 89, 188, 180, 175, 88, 235, 141,
                    245, 169, 16, 67, 84, 12, 19, 152, 221, 152, 122, 80, 49, 39, 252
                ]
        );
    }
}
