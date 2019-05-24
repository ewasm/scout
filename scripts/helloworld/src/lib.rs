extern crate ewasm_api;

use ewasm_api::*;

#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main() {
    let pre_state = eth2::load_pre_state();

    assert!(eth2::block_data_size() == 0);

    // No updates were made to the state
    let post_state = pre_state;

    eth2::save_post_state(post_state)
}
