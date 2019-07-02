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
use ewasm_api::*;
use hash_db::{HashDB, EMPTY_PREFIX};
use kvdb::DBValue;
use memory_db::*;
use rlp::{DecoderError, Rlp};
use rlp_node_codec::RlpNodeCodec;
use sig::recover_address;
use tiny_keccak::keccak256;
use trie::{Trie, TrieMut};
use tx::{Tx, UnsignedTx};

type RlpCodec = RlpNodeCodec<KeccakHasher>;
type SecTrieDB<'db> = trie::SecTrieDB<'db, KeccakHasher, RlpCodec>;
type SecTrieDBMut<'db> = trie::SecTrieDBMut<'db, KeccakHasher, RlpCodec>;

#[derive(Debug, Clone, PartialEq, Eq)]
struct BlockData {
    tx: Tx,
    from_witness: Vec<Vec<u8>>,
    to_witness: Vec<Vec<u8>>,
}

impl rlp::Decodable for BlockData {
    fn decode(d: &Rlp) -> Result<Self, DecoderError> {
        if d.item_count()? != 3 {
            return Err(DecoderError::RlpIncorrectListLen);
        }

        Ok(BlockData {
            tx: d.val_at(0)?,
            from_witness: d.list_at(1)?,
            to_witness: d.list_at(2)?,
        })
    }
}

fn process_block(pre_state_root: types::Bytes32, block_data_bytes: &[u8]) -> types::Bytes32 {
    let block_data: BlockData = rlp::decode(&block_data_bytes).unwrap();
    let tx = block_data.tx;

    // Recover sender from signature
    let tx_rlp = rlp::encode(&UnsignedTx {
        to: tx.to,
        value: tx.value,
        nonce: tx.nonce,
    });
    let tx_hash = keccak256(&tx_rlp);
    let from_address = recover_address(&tx.sig, tx_hash);

    // Insert witnesses to trie's underlying db
    // and construct trie from witnesses
    let mut db = MemoryDB::<KeccakHasher, HashKey<_>, DBValue>::from_null_node(
        &rlp::NULL_RLP,
        rlp::NULL_RLP.as_ref().into(),
    );
    for item in block_data.from_witness {
        db.insert(EMPTY_PREFIX, item.as_slice());
    }
    for item in block_data.to_witness {
        db.insert(EMPTY_PREFIX, item.as_slice());
    }
    let mut root = H256::from_slice(&pre_state_root.bytes[..]);
    let t = SecTrieDB::new(&db, &root).unwrap();

    // Make sure trie contains `from` and `to`
    assert!(t.contains(from_address.as_bytes()).unwrap());
    assert!(t.contains(tx.to.as_bytes()).unwrap());

    // Fetch `from` and `to` accounts from trie
    let from_account_bytes = t.get(from_address.as_bytes()).unwrap().unwrap();
    let mut from_account = rlp::decode::<BasicAccount>(&from_account_bytes).unwrap();
    let to_account_bytes = t.get(tx.to.as_bytes()).unwrap().unwrap();
    let mut to_account = rlp::decode::<BasicAccount>(&to_account_bytes).unwrap();

    // Pre transfer checks
    assert!(from_account.nonce == tx.nonce);
    assert!(from_account.balance >= tx.value);

    // Increment sender's nonce and update balances
    from_account.nonce += U256::from(1);
    from_account.balance -= tx.value;
    to_account.balance += tx.value;

    // Update trie with new balances and nonces
    let mut mt = SecTrieDBMut::from_existing(&mut db, &mut root).unwrap();
    let from_encoded = rlp::encode(&from_account);
    let to_encoded = rlp::encode(&to_account);
    mt.insert(from_address.as_bytes(), &from_encoded).unwrap();
    mt.insert(tx.to.as_bytes(), &to_encoded).unwrap();
    let post_root = mt.root();

    types::Bytes32::from(post_root.as_fixed_bytes())
}

#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main() {
    let pre_state_root = eth2::load_pre_state_root();
    let block_data = eth2::acquire_block_data();
    let post_state_root = process_block(pre_state_root, &block_data);
    eth2::save_post_state_root(post_state_root)
}
