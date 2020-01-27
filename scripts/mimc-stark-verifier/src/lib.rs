extern crate stark_verifier;
extern crate ewasm_api;
extern crate num_bigint;

use std::str::FromStr;
use num_traits::pow::Pow;

use ewasm_api::*;
use stark_verifier::deserializer;
use stark_verifier::{verify_mimc_proof, MODULUS};
use num_bigint::{BigInt, BigUint};

fn process_block(pre_state_root: types::Bytes32, block_data: &[u8]) -> types::Bytes32 {
    let (stark_proof, _) = deserializer::from_bytes(&block_data).expect("couldn't deserialize");

    // TODO: package subsequent parameters in the proof itself
    const LOG_STEPS: usize = 13;
    let mut constants: Vec<BigInt> = Vec::new();
    let modulus: BigInt = num_bigint::BigInt::from_str(MODULUS).expect("modulus couldn't be deserialized into bigint");

    for i in 0..64 {
        let constant = BigInt::from(i as u8).pow(BigUint::from(7u8)) ^ BigInt::from(42u8);
        constants.push(constant);
    }

    let output = BigInt::from_str("95224774355499767951968048714566316597785297695903697235130434363122555476056").unwrap();

    match verify_mimc_proof(BigInt::from(3u8), 2usize.pow(LOG_STEPS as u32), &constants, output, stark_proof, &modulus) {
        true => types::Bytes32 { bytes: [0u8; 32] },
        false => types::Bytes32 { bytes: [1u8; 32] }
    }
}

#[no_mangle]
pub extern "C" fn main() {
    let pre_state_root = eth2::load_pre_state_root();
    let block_data = eth2::acquire_block_data();
    let post_state_root = process_block(pre_state_root, &block_data);
    eth2::save_post_state_root(post_state_root)
}
