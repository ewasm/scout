extern crate ewasm_api;

use ewasm_api::*;

#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main() {
    let pre_state_root = eth2::load_pre_state_root();

    assert!(eth2::block_data_size() == 0);

    // No updates were made to the state
    let post_state_root = pre_state_root;

    eth2::save_post_state_root(&post_state_root)
}
