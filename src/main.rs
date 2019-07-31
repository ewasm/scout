extern crate rustc_hex;
extern crate wasmi;
#[macro_use]
extern crate log;
extern crate env_logger;

use rustc_hex::{FromHex, ToHex};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs::File;
use wasmi::memory_units::Pages;
use wasmi::{
    Error as InterpreterError, Externals, FuncInstance, FuncRef, ImportsBuilder, MemoryInstance,
    MemoryRef, Module, ModuleImportResolver, ModuleInstance, NopExternals, RuntimeArgs,
    RuntimeValue, Signature, Trap, TrapKind, ValueType,
};

mod types;
use crate::types::*;

const LOADPRESTATEROOT_FUNC_INDEX: usize = 0;
const BLOCKDATASIZE_FUNC_INDEX: usize = 1;
const BLOCKDATACOPY_FUNC_INDEX: usize = 2;
const SAVEPOSTSTATEROOT_FUNC_INDEX: usize = 3;
const PUSHNEWDEPOSIT_FUNC_INDEX: usize = 4;
const USETICKS_FUNC_INDEX: usize = 5;
const DEBUG_PRINT32_FUNC: usize = 6;
const DEBUG_PRINT64_FUNC: usize = 7;
const DEBUG_PRINTMEM_FUNC: usize = 8;
const DEBUG_PRINTMEMHEX_FUNC: usize = 9;

struct Runtime<'a> {
    ticks_left: u32,
    memory: Option<MemoryRef>,
    pre_state: &'a Bytes32,
    block_data: &'a ShardBlockBody,
    post_state: Bytes32,
}

impl<'a> Runtime<'a> {
    fn new(
        pre_state: &'a Bytes32,
        block_data: &'a ShardBlockBody,
        memory: Option<MemoryRef>,
    ) -> Runtime<'a> {
        Runtime {
            ticks_left: 10_000_000, // FIXME: make this configurable
            memory: if memory.is_some() {
                memory
            } else {
                // Allocate a single page if no memory was exported.
                Some(MemoryInstance::alloc(Pages(1), Some(Pages(1))).unwrap())
            },
            pre_state: pre_state,
            block_data: block_data,
            post_state: Bytes32::default(),
        }
    }

    fn get_post_state(&self) -> Bytes32 {
        self.post_state
    }
}

impl<'a> Externals for Runtime<'a> {
    fn invoke_index(
        &mut self,
        index: usize,
        args: RuntimeArgs,
    ) -> Result<Option<RuntimeValue>, Trap> {
        match index {
            USETICKS_FUNC_INDEX => {
                let ticks: u32 = args.nth(0);
                if self.ticks_left < ticks {
                    // FIXME: use TrapKind::Host
                    return Err(Trap::new(TrapKind::Unreachable));
                }
                self.ticks_left -= ticks;
                Ok(None)
            }
            LOADPRESTATEROOT_FUNC_INDEX => {
                let ptr: u32 = args.nth(0);
                info!("loadprestateroot to {}", ptr);

                // TODO: add checks for out of bounds access
                let memory = self.memory.as_ref().expect("expects memory object");
                memory
                    .set(ptr, &self.pre_state.bytes)
                    .expect("expects writing to memory to succeed");

                Ok(None)
            }
            SAVEPOSTSTATEROOT_FUNC_INDEX => {
                let ptr: u32 = args.nth(0);
                info!("savepoststateroot from {}", ptr);

                // TODO: add checks for out of bounds access
                let memory = self.memory.as_ref().expect("expects memory object");
                memory
                    .get_into(ptr, &mut self.post_state.bytes)
                    .expect("expects reading from memory to succeed");

                Ok(None)
            }
            BLOCKDATASIZE_FUNC_INDEX => {
                let ret: i32 = self.block_data.data.len() as i32;
                info!("blockdatasize {}", ret);
                Ok(Some(ret.into()))
            }
            BLOCKDATACOPY_FUNC_INDEX => {
                let ptr: u32 = args.nth(0);
                let offset: u32 = args.nth(1);
                let length: u32 = args.nth(2);
                info!(
                    "blockdatacopy to {} from {} for {} bytes",
                    ptr, offset, length
                );

                // TODO: add overflow check
                let offset = offset as usize;
                let length = length as usize;

                // TODO: add checks for out of bounds access
                let memory = self.memory.as_ref().expect("expects memory object");
                memory
                    .set(ptr, &self.block_data.data[offset..length])
                    .expect("expects writing to memory to succeed");

                Ok(None)
            }
            PUSHNEWDEPOSIT_FUNC_INDEX => unimplemented!(),
            DEBUG_PRINT32_FUNC => {
                let value: u32 = args.nth(0);
                debug!("print.i32: {}", value);
                Ok(None)
            }
            DEBUG_PRINT64_FUNC => {
                let value: u64 = args.nth(0);
                debug!("print.i64: {}", value);
                Ok(None)
            }
            DEBUG_PRINTMEM_FUNC => {
                let ptr: u32 = args.nth(0);
                let length: u32 = args.nth(1);
                let mut buf = Vec::with_capacity(length as usize);
                unsafe { buf.set_len(length as usize) };
                // TODO: add checks for out of bounds access
                let memory = self.memory.as_ref().expect("expects memory object");
                memory
                    .get_into(ptr, &mut buf)
                    .expect("expects reading from memory to succeed");
                debug!("print: {}", String::from_utf8_lossy(&buf));
                Ok(None)
            }
            DEBUG_PRINTMEMHEX_FUNC => {
                let ptr: u32 = args.nth(0);
                let length: u32 = args.nth(1);
                let mut buf = Vec::with_capacity(length as usize);
                unsafe { buf.set_len(length as usize) };
                // TODO: add checks for out of bounds access
                let memory = self.memory.as_ref().expect("expects memory object");
                memory
                    .get_into(ptr, &mut buf)
                    .expect("expects reading from memory to succeed");
                debug!("print.hex: {}", buf.to_hex());
                Ok(None)
            }
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
            "eth2_useTicks" => FuncInstance::alloc_host(
                Signature::new(&[ValueType::I32][..], None),
                USETICKS_FUNC_INDEX,
            ),
            "eth2_loadPreStateRoot" => FuncInstance::alloc_host(
                Signature::new(&[ValueType::I32][..], None),
                LOADPRESTATEROOT_FUNC_INDEX,
            ),
            "eth2_blockDataSize" => FuncInstance::alloc_host(
                Signature::new(&[][..], Some(ValueType::I32)),
                BLOCKDATASIZE_FUNC_INDEX,
            ),
            "eth2_blockDataCopy" => FuncInstance::alloc_host(
                Signature::new(&[ValueType::I32, ValueType::I32, ValueType::I32][..], None),
                BLOCKDATACOPY_FUNC_INDEX,
            ),
            "eth2_savePostStateRoot" => FuncInstance::alloc_host(
                Signature::new(&[ValueType::I32][..], None),
                SAVEPOSTSTATEROOT_FUNC_INDEX,
            ),
            "eth2_pushNewDeposit" => FuncInstance::alloc_host(
                Signature::new(&[ValueType::I32, ValueType::I32][..], None),
                PUSHNEWDEPOSIT_FUNC_INDEX,
            ),
            "debug_print32" => FuncInstance::alloc_host(
                Signature::new(&[ValueType::I32][..], None),
                DEBUG_PRINT32_FUNC,
            ),
            "debug_print64" => FuncInstance::alloc_host(
                Signature::new(&[ValueType::I64][..], None),
                DEBUG_PRINT64_FUNC,
            ),
            "debug_printMem" => FuncInstance::alloc_host(
                Signature::new(&[ValueType::I32, ValueType::I32][..], None),
                DEBUG_PRINTMEM_FUNC,
            ),
            "debug_printMemHex" => FuncInstance::alloc_host(
                Signature::new(&[ValueType::I32, ValueType::I32][..], None),
                DEBUG_PRINTMEMHEX_FUNC,
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

const BYTES_PER_SHARD_BLOCK_BODY: usize = 16384;
const ZERO_HASH: Bytes32 = Bytes32 { bytes: [0u8; 32] };

/// These are Phase 0 structures.
/// https://github.com/ethereum/eth2.0-specs/blob/dev/specs/core/0_beacon-chain.md
#[derive(Default, PartialEq, Clone, Debug)]
pub struct Deposit {}

/// These are Phase 2 Proposal 2 structures.

#[derive(Default, PartialEq, Clone, Debug)]
pub struct ExecutionScript {
    code: Vec<u8>,
}

#[derive(Default, PartialEq, Clone, Debug)]
pub struct BeaconState {
    execution_scripts: Vec<ExecutionScript>,
}

/// Shards are Phase 1 structures.
/// https://github.com/ethereum/eth2.0-specs/blob/dev/specs/core/1_shard-data-chains.md

#[derive(Default, PartialEq, Clone, Debug)]
pub struct ShardBlockHeader {}

#[derive(Default, PartialEq, Clone, Debug)]
pub struct ShardBlockBody {
    data: Vec<u8>,
}

#[derive(Default, PartialEq, Clone, Debug)]
pub struct ShardBlock {
    env: u64, // This is added by Phase 2 Proposal 2
    data: ShardBlockBody,
    // TODO: add missing fields
}

#[derive(Default, PartialEq, Clone, Debug)]
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
    debug!(
        "Executing codesize({}) and data: {:#?}",
        code.len(),
        block_data
    );

    let module = Module::from_buffer(&code).expect("Module loading to succeed");
    let mut imports = ImportsBuilder::new();
    // FIXME: use eth2
    imports.push_resolver("env", &RuntimeModuleImportResolver);

    let instance = ModuleInstance::new(&module, &imports)
        .expect("Module instantation expected to succeed")
        .run_start(&mut NopExternals)
        .expect("Failed to run start function in module");

    let internal_mem = instance
        .export_by_name("memory")
        .expect("Module expected to have 'memory' export")
        .as_memory()
        .cloned()
        .expect("'memory' export should be a memory");

    let mut runtime = Runtime::new(pre_state, block_data, Some(internal_mem));

    let result = instance
        .invoke_export("main", &[], &mut runtime)
        .expect("Executed 'main'");

    info!("Result: {:?}", result);
    info!("Execution finished");

    (runtime.get_post_state(), vec![Deposit {}])
}

pub fn process_shard_block(
    state: &mut ShardState,
    beacon_state: &BeaconState,
    block: Option<ShardBlock>,
) {
    // println!("Beacon state: {:#?}", beacon_state);
    info!("Executing block: {:#?}", block);

    info!("Pre-execution: {:#?}", state);

    // TODO: implement state root handling

    if let Some(block) = block {
        // The execution environment identifier
        let env = block.env as usize; // FIXME: usize can be 32-bit
        let code = &beacon_state.execution_scripts[env].code;

        // Set post states to empty for any holes
        // for x in 0..env {
        //     state.exec_env_states.push(ZERO_HASH)
        // }
        let pre_state = &state.exec_env_states[env];
        let (post_state, deposits) = execute_code(code, pre_state, &block.data);
        state.exec_env_states[env] = post_state
    }

    // TODO: implement state + deposit root handling

    info!("Post-execution: {:#?}", state)
}

fn load_file(filename: &str) -> Vec<u8> {
    use std::io::prelude::*;
    let mut file = File::open(filename).expect(&format!("loading file {} failed", filename));
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)
        .expect(&format!("reading file {} failed", filename));
    buf
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct TestBeaconState {
    execution_scripts: Vec<String>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct TestShardBlock {
    env: u64,
    data: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct TestShardState {
    exec_env_states: Vec<String>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct TestFile {
    beacon_state: TestBeaconState,
    shard_blocks: Vec<TestShardBlock>,
    shard_pre_state: TestShardState,
    shard_post_state: TestShardState,
}

impl From<TestBeaconState> for BeaconState {
    fn from(input: TestBeaconState) -> Self {
        BeaconState {
            execution_scripts: input
                .execution_scripts
                .iter()
                .map(|x| ExecutionScript { code: load_file(x) })
                .collect(),
        }
    }
}

impl From<TestShardBlock> for ShardBlock {
    fn from(input: TestShardBlock) -> Self {
        ShardBlock {
            env: input.env,
            data: ShardBlockBody {
                data: input.data.from_hex().expect("invalid hex data"),
            },
        }
    }
}

impl From<TestShardState> for ShardState {
    fn from(input: TestShardState) -> Self {
        ShardState {
            exec_env_states: input
                .exec_env_states
                .iter()
                .map(|x| {
                    let state = x.from_hex().expect("invalid hex data");
                    assert!(state.len() == 32);
                    let mut ret = Bytes32::default();
                    ret.bytes.copy_from_slice(&state[..]);
                    ret
                })
                .collect(),
            slot: 0,
            parent_block: ShardBlockHeader {},
        }
    }
}

fn process_yaml_test(filename: &str) {
    info!("Processing {}...", filename);
    let content = load_file(&filename);
    let test_file: TestFile =
        serde_yaml::from_slice::<TestFile>(&content[..]).expect("expected valid yaml");
    debug!("{:#?}", test_file);

    let beacon_state: BeaconState = test_file.beacon_state.into();
    let pre_state: ShardState = test_file.shard_pre_state.into();
    let post_state: ShardState = test_file.shard_post_state.into();

    let mut shard_state = pre_state;
    for block in test_file.shard_blocks {
        process_shard_block(&mut shard_state, &beacon_state, Some(block.into()))
    }
    debug!("{:#?}", shard_state);
    assert_eq!(shard_state, post_state);
}

fn main() {
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    process_yaml_test(if args.len() != 2 {
        "test.yaml"
    } else {
        &args[1]
    });
}
