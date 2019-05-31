use ewasm_api::*;

#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main() {
    let ret = [1u8, 5];
    let _ = eth2::acquire_block_data();
    eth2::save_return_data(&ret);
}
