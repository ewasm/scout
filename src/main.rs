const BYTES_PER_SHARD_BLOCK_BODY: usize = 16384;
const ZERO_HASH: Bytes32 = Bytes32 {};

/// These are Phase 0 structures.
/// https://github.com/ethereum/eth2.0-specs/blob/dev/specs/core/0_beacon-chain.md

#[derive(Default, Clone, Debug)]
pub struct Bytes32 {}

#[derive(Default, Clone, Debug)]
pub struct Deposit {}

/// These are Phase 2 Proposal 2 structures.

#[derive(Default, Clone, Debug)]
pub struct ExecutionScript {
    code: Vec<u8>,
}

#[derive(Default, Clone, Debug)]
pub struct BeaconState {
    execution_scripts: Vec<ExecutionScript>,
}

/// Shards are Phase 1 structures.
/// https://github.com/ethereum/eth2.0-specs/blob/dev/specs/core/1_shard-data-chains.md

#[derive(Default, Clone, Debug)]
pub struct ShardBlockHeader {}

#[derive(Default, Clone, Debug)]
pub struct ShardBlockBody {
    data: Vec<u8>,
}

#[derive(Default, Clone, Debug)]
pub struct ShardBlock {
    env: u64, // This is added by Phase 2 Proposal 2
    data: ShardBlockBody,
    // TODO: add missing fields
}

#[derive(Default, Clone, Debug)]
pub struct ShardState {
    exec_env_states: Vec<Bytes32>,
    slot: u64,
    parent_block: ShardBlockHeader,
    // TODO: add missing field
    // latest_state_roots: [bytes32, LATEST_STATE_ROOTS_LEMGTH]
}

pub fn execute_code(
    code: &[u8],
    pre_state: &Bytes32,
    block_data: &ShardBlockBody,
) -> (Bytes32, Vec<Deposit>) {
    println!("Executing code: {:#?} with data {:#?}", code, block_data);
    (Bytes32 {}, vec![Deposit {}])
}

pub fn process_shard_block(
    state: &mut ShardState,
    beacon_state: BeaconState,
    block: Option<ShardBlock>,
) {
    println!("Beacon state: {:#?}", beacon_state);
    println!("Executing block: {:#?}", block);

    println!("Pre-execution: {:#?}", state);

    // TODO: implement state root handling

    if let Some(block) = block {
        // The execution environment identifier
        let env = block.env as usize; // FIXME: usize can be 32-bit
        let code = &beacon_state.execution_scripts[env].code;

        // Set post states to empty for any holes
        for x in 0..env {
            state.exec_env_states.push(ZERO_HASH)
        }
        let pre_state = &state.exec_env_states[env];
        let (post_state, deposits) = execute_code(code, pre_state, &block.data);
        state.exec_env_states[env] = post_state
    }

    // TODO: implement state + deposit root handling

    println!("Post-execution: {:#?}", state)
}

fn main() {
    let mut shard_state = ShardState {
        exec_env_states: vec![Bytes32 {}],
        slot: 0,
        parent_block: ShardBlockHeader {},
    };
    let beacon_state = BeaconState {
        execution_scripts: vec![
            ExecutionScript {
                code: [0u8; 1].to_vec(),
            },
            ExecutionScript {
                code: [0u8; 1].to_vec(),
            },
        ],
    };
    let shard_block = ShardBlock {
        env: 1,
        data: ShardBlockBody { data: vec![] },
    };
    process_shard_block(&mut shard_state, beacon_state, Some(shard_block))
}
