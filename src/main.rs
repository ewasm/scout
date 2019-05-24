extern crate rustc_hex;
extern crate wasmi;

use rustc_hex::FromHex;
use std::env::args;
use std::fs::File;
use wasmi::memory_units::Pages;
use wasmi::{
    Error as InterpreterError, Externals, FuncInstance, FuncRef, ImportsBuilder, MemoryInstance,
    MemoryRef, Module, ModuleImportResolver, ModuleInstance, NopExternals, RuntimeArgs,
    RuntimeValue, Signature, Trap, ValueType,
};

mod types;
use crate::types::*;

const USEGAS_FUNC_INDEX: usize = 0;

struct Runtime {
    memory: Option<MemoryRef>,
}

impl Runtime {
    fn new() -> Runtime {
        Runtime {
            memory: Some(MemoryInstance::alloc(Pages(1), Some(Pages(1))).unwrap()),
        }
    }
}

impl<'a> Externals for Runtime {
    fn invoke_index(
        &mut self,
        index: usize,
        args: RuntimeArgs,
    ) -> Result<Option<RuntimeValue>, Trap> {
        match index {
            USEGAS_FUNC_INDEX => Ok(None),
            _ => panic!("unknown function index"),
        }
    }
}

struct RuntimeModuleImportResolver;

impl<'a> ModuleImportResolver for RuntimeModuleImportResolver {
    fn resolve_func(
        &self,
        field_name: &str,
        _signature: &Signature,
    ) -> Result<FuncRef, InterpreterError> {
        let func_ref = match field_name {
            "useGas" => FuncInstance::alloc_host(
                Signature::new(&[ValueType::I64][..], None),
                USEGAS_FUNC_INDEX,
            ),
            _ => {
                return Err(InterpreterError::Function(format!(
                    "host module doesn't export function with name {}",
                    field_name
                )))
            }
        };
        Ok(func_ref)
    }
}

fn wasm_load_from_file(filename: &str) -> Module {
    use std::io::prelude::*;
    let mut file = File::open(filename).unwrap();
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).unwrap();
    Module::from_buffer(buf).unwrap()
}

fn wasm_load_from_blob(buf: &[u8]) -> Module {
    Module::from_buffer(buf).unwrap()
}

const BYTES_PER_SHARD_BLOCK_BODY: usize = 16384;
const ZERO_HASH: Bytes32 = Bytes32 { bytes: [0u8; 32] };

/// These are Phase 0 structures.
/// https://github.com/ethereum/eth2.0-specs/blob/dev/specs/core/0_beacon-chain.md
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
    println!(
        "Executing codesize({}) and data: {:#?}",
        code.len(),
        block_data
    );

    let module = wasm_load_from_blob(&code);
    let mut imports = ImportsBuilder::new();
    imports.push_resolver("ethereum", &RuntimeModuleImportResolver);

    let instance = ModuleInstance::new(&module, &imports)
        .unwrap()
        .assert_no_start();

    let mut runtime = Runtime::new();

    let result = instance
        .invoke_export("main", &[], &mut runtime)
        .expect("Executed 'main'");

    println!("Result: {:?}", result);
    println!("Execution finished");

    (Bytes32::default(), vec![Deposit {}])
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
    let execution_script = FromHex::from_hex(
        "
            0061736d010000000113046000017f60037f7f7f0060027f7f00600000023e0308
            657468657265756d0b676574436f646553697a65000008657468657265756d0863
            6f6465436f7079000108657468657265756d0666696e6973680002030201030503
            010001071102066d656d6f72790200046d61696e00030a2c012a01037f10002100
            4100410020001001200041046b2802002102200041046b20026b21012001200210
            020b

            000d086465706c6f79657200000000
        ",
    )
    .unwrap();

    let mut shard_state = ShardState {
        exec_env_states: vec![Bytes32::default()],
        slot: 0,
        parent_block: ShardBlockHeader {},
    };
    let beacon_state = BeaconState {
        execution_scripts: vec![
            ExecutionScript {
                code: execution_script.to_vec(),
            },
            ExecutionScript {
                code: execution_script.to_vec(),
            },
        ],
    };
    let shard_block = ShardBlock {
        env: 1,
        data: ShardBlockBody { data: vec![] },
    };
    process_shard_block(&mut shard_state, beacon_state, Some(shard_block))
}
