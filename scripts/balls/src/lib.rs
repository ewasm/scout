use ewasm_api::eth2;
use pairing::bls12_381::Bls12;
use pairing::bls12_381::FrRepr;
use pairing::bls12_381::G1Uncompressed;
use pairing::bls12_381::G2Uncompressed;
use pairing::CurveAffine;
use pairing::EncodedPoint;
use pairing::Engine;
use ssz::Decode;
use ssz_derive::Ssz;

// Dummy
const HASH_KEY: &[u8] = b"BLSSignatureSeed";

#[derive(Debug, PartialEq, Ssz, Default)]
struct InputBlock {
    pub pubkey: Vec<u8>,
    pub message: Vec<u8>,
    pub signature: Vec<u8>,
}

fn verify_signature(
    message: &[u8],
    signature: <Bls12 as Engine>::G1Affine,
    pubkey: <Bls12 as Engine>::G2Affine,
) -> bool {
    // Hash the message to a curve point.
    let hashed_to_curve = <Bls12 as Engine>::G1Affine::hash(HASH_KEY, message);

    // If the pairing of the signature and the generator for G2 is equal to the pairing of the
    // hash of the message and the public key, then the signature is valid.
    // e(sig, g2) == e(H(msg), pk)
    Bls12::pairing(signature, <Bls12 as Engine>::G2Affine::one())
        == Bls12::pairing(hashed_to_curve, pubkey)
}

#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main() {
    let pre_state = eth2::load_pre_state_root();
    let mut post_state = pre_state;
    // Set last byte zero, only set 1 on successful verification
    post_state.bytes[31] = 0;

    if eth2::block_data_size() != 0 {
        let block_data = eth2::acquire_block_data();
        let mut block_data: &[u8] = &block_data;
        let input = InputBlock::decode(&mut block_data).expect("SSZ decoding failure");

        // Verify that the sizes of the signature and public key are correct.
        assert_eq!(input.signature.len(), 96);
        assert_eq!(input.pubkey.len(), 192);

        // Copy these into static arrays so we can move them into uncompressed point types
        let mut sig_buf = [0u8; 96];
        let mut pubkey_buf = [0u8; 192];
        sig_buf.copy_from_slice(&input.signature[..96]);
        pubkey_buf.copy_from_slice(&input.pubkey[..192]);

        // Move into uncompressed encoded point types
        let mut signature_uncompressed = G1Uncompressed { sig: sig_buf };
        let mut pubkey_uncompressed = G2Uncompressed { pubkey: pubkey_buf };

        // Convert to affine curve group elements
        let signature = signature_uncompressed
            .into_affine()
            .expect("Encoded point does not represent a valid group element: signature");
        let pubkey = pubkey_uncompressed
            .into_affine()
            .expect("Encoded point does not represent a valid group element: pubkey");

        //let signature = <Bls12 as Engine>::G1::from(signature_affine);
        //let pubkey = <Bls12 as Engine>::G2::from(pubkey_affine);

        if verify_signature(&input.message, signature, pubkey) {
            // Set last byte if verified successfully
            post_state.bytes[31] = 1;
        }
    }
    eth2::save_post_state_root(&post_state);
}
