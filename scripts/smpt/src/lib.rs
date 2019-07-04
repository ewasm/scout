extern crate ethereum_types;
extern crate ewasm_api;
extern crate hash_db;
extern crate memory_db;
extern crate patricia_trie_ethereum as ethtrie;
extern crate secp256k1;
extern crate trie_db as trie;

mod account;
mod keccak_hasher;
mod rlp_node_codec;
mod sig;
mod tx;

use crate::keccak_hasher::KeccakHasher;
use account::BasicAccount;
use ethereum_types::{H256, U256};
use ewasm_api::prelude::*;
use hash_db::{HashDB, EMPTY_PREFIX};
use kvdb::DBValue;
use memory_db::*;
use rlp::{DecoderError, Rlp};
use rlp_node_codec::RlpNodeCodec;
use sig::recover_address;
use tiny_keccak::keccak256;
use trie::TrieMut;
use tx::{StatefulTx, UnsignedTx};

type RlpCodec = RlpNodeCodec<KeccakHasher>;
type SecTrieDBMut<'db> = trie::SecTrieDBMut<'db, KeccakHasher, RlpCodec>;

#[derive(Debug, Clone, PartialEq, Eq)]
struct BlockData {
    txes: Vec<StatefulTx>,
}

impl rlp::Decodable for BlockData {
    fn decode(d: &Rlp) -> Result<Self, DecoderError> {
        if d.item_count()? != 1 {
            return Err(DecoderError::RlpIncorrectListLen);
        }

        Ok(BlockData {
            txes: d.list_at(0)?,
        })
    }
}

fn process_block(pre_state_root: Bytes32, block_data_bytes: &[u8]) -> Bytes32 {
    let stateful_txes: Vec<StatefulTx> = rlp::decode_list(&block_data_bytes);

    // Construct trie from merkle proofs
    let mut db = MemoryDB::<KeccakHasher, HashKey<_>, DBValue>::from_null_node(
        &rlp::NULL_RLP,
        rlp::NULL_RLP.as_ref().into(),
    );
    for tx in stateful_txes.clone() {
        // Insert proof values to trie's underlying db
        for item in tx.from_witness {
            db.insert(EMPTY_PREFIX, item.as_slice());
        }
        for item in tx.to_witness {
            db.insert(EMPTY_PREFIX, item.as_slice());
        }
    }

    let mut root = H256::from_slice(&pre_state_root.bytes[..]);
    let mut trie = SecTrieDBMut::from_existing(&mut db, &mut root).unwrap();

    for stateful_tx in stateful_txes {
        let tx = stateful_tx.tx;
        // Recover sender from signature
        let tx_rlp = rlp::encode(&UnsignedTx {
            to: tx.to,
            value: tx.value,
            nonce: tx.nonce,
        });
        let tx_hash = keccak256(&tx_rlp);
        let from_address = recover_address(&tx.sig, tx_hash);

        // Make sure trie contains `from` and `to`
        assert!(trie.contains(from_address.as_bytes()).unwrap());
        assert!(trie.contains(tx.to.as_bytes()).unwrap());

        // Fetch `from` and `to` accounts from trie
        let from_account_bytes = trie.get(from_address.as_bytes()).unwrap().unwrap();
        let mut from_account = rlp::decode::<BasicAccount>(&from_account_bytes).unwrap();
        let to_account_bytes = trie.get(tx.to.as_bytes()).unwrap().unwrap();
        let mut to_account = rlp::decode::<BasicAccount>(&to_account_bytes).unwrap();

        // Pre transfer checks
        assert!(from_account.nonce == tx.nonce);
        assert!(from_account.balance >= tx.value);

        // Increment sender's nonce and update balances
        from_account.nonce += U256::from(1);
        from_account.balance -= tx.value;
        to_account.balance += tx.value;

        // Update trie with new balances and nonces
        let from_encoded = rlp::encode(&from_account);
        let to_encoded = rlp::encode(&to_account);
        trie.insert(from_address.as_bytes(), &from_encoded).unwrap();
        trie.insert(tx.to.as_bytes(), &to_encoded).unwrap();
    }

    Bytes32::from(trie.root().as_fixed_bytes())
}

#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main() {
    let pre_state_root = eth2::load_pre_state_root();
    let block_data = eth2::acquire_block_data();
    let post_state_root = process_block(pre_state_root, &block_data);
    eth2::save_post_state_root(&post_state_root)
}
