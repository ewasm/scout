use crate::sig::Sig;
use ethereum_types::{Address, U256};
use rlp::{Decodable, DecoderError, Encodable, Rlp, RlpStream};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatefulTx {
    pub tx: Tx,
    pub from_witness: Vec<Vec<u8>>,
    pub to_witness: Vec<Vec<u8>>,
}

impl Decodable for StatefulTx {
    fn decode(d: &Rlp) -> Result<Self, DecoderError> {
        if d.item_count()? != 3 {
            return Err(DecoderError::RlpIncorrectListLen);
        }

        Ok(StatefulTx {
            tx: d.val_at(0)?,
            from_witness: d.list_at(1)?,
            to_witness: d.list_at(2)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tx {
    pub to: Address,
    pub value: U256,
    pub nonce: U256,
    pub sig: Sig,
}

impl Decodable for Tx {
    fn decode(d: &Rlp) -> Result<Self, DecoderError> {
        if d.item_count()? != 4 {
            return Err(DecoderError::RlpIncorrectListLen);
        }

        Ok(Tx {
            to: d.val_at(0)?,
            value: d.val_at(1)?,
            nonce: d.val_at(2)?,
            sig: d.val_at(3)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsignedTx {
    pub to: Address,
    pub value: U256,
    pub nonce: U256,
}

impl Encodable for UnsignedTx {
    fn rlp_append(&self, s: &mut RlpStream) {
        s.begin_list(3);
        s.append(&self.to);
        s.append(&self.value);
        s.append(&self.nonce);
    }
}
