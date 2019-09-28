extern crate ewasm_api;

use ewasm_api::*;

#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main() {
    let pre_state_root = eth2::load_pre_state_root();

    // Show debug functionality
    debug::log("hello world!");
    debug::print32(42);
    debug::print64(99);
    debug::print_mem(&pre_state_root.bytes);
    debug::print_mem_hex(&pre_state_root.bytes);

    assert!(eth2::block_data_size() == 0);

    // No updates were made to the state
    let post_state_root = pre_state_root;

    // Deposit only for demo purposes
    // This is valid length-wise, but invalid content-wise
    let deposit = [0u8; 184];
    eth2::push_new_deposit(&deposit);

    eth2::save_post_state_root(&post_state_root)
}
