use ethereum_types::{Address, U256};
use rlp::{DecoderError, Rlp};
use secp256k1::{recover, Message, RecoveryId, Signature};
use tiny_keccak::keccak256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sig {
    pub r: U256,
    pub s: U256,
    pub v: u8,
}

impl rlp::Decodable for Sig {
    fn decode(d: &Rlp) -> Result<Self, DecoderError> {
        if d.item_count()? != 3 {
            return Err(DecoderError::RlpIncorrectListLen);
        }

        Ok(Sig {
            r: d.val_at(0)?,
            s: d.val_at(1)?,
            v: d.val_at(2)?,
        })
    }
}

pub fn recover_address(signature: &Sig, message: [u8; 32]) -> Address {
    let mut s = [0u8; 64];
    (&mut s[..32]).copy_from_slice(&Into::<[u8; 32]>::into(signature.r)[..32]);
    (&mut s[32..64]).copy_from_slice(&Into::<[u8; 32]>::into(signature.s)[..32]);

    let message = Message::parse(&message);
    let rec_id = RecoveryId::parse(signature.v - 27).unwrap();
    let sig = Signature::parse(&s);

    let key = recover(&message, &sig, &rec_id).unwrap();
    let ret = key.serialize();
    let ret = keccak256(&ret[1..65]);

    Address::from_slice(&ret[12..32])
}
