use ethereum_types::{H256, U256};
use rlp::{Decodable, DecoderError, Encodable, Rlp, RlpStream};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasicAccount {
    pub nonce: U256,
    pub balance: U256,
    pub storage_root: H256,
    pub code_hash: H256,
}

impl Decodable for BasicAccount {
    fn decode(d: &Rlp) -> Result<Self, DecoderError> {
        if d.item_count()? != 4 {
            return Err(DecoderError::RlpIncorrectListLen);
        }

        Ok(BasicAccount {
            nonce: d.val_at(0)?,
            balance: d.val_at(1)?,
            storage_root: d.val_at(2)?,
            code_hash: d.val_at(3)?,
        })
    }
}

impl Encodable for BasicAccount {
    fn rlp_append(&self, s: &mut RlpStream) {
        s.begin_list(4);
        s.append(&self.nonce);
        s.append(&self.balance);
        s.append(&self.storage_root);
        s.append(&self.code_hash);
    }
}
