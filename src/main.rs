extern crate rustc_hex;
extern crate wasmi;

use rustc_hex::FromHex;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};
use sszt::yaml::to_ssz;
use std::cell::RefCell;
use std::env;
use std::fs::File;
use std::rc::Rc;
use wasmi::memory_units::Pages;
use wasmi::{
    Error as InterpreterError, Externals, FuncInstance, FuncRef, ImportsBuilder, MemoryInstance,
    MemoryRef, Module, ModuleImportResolver, ModuleInstance, RuntimeArgs, RuntimeValue, Signature,
    Trap, ValueType,
};

mod types;
use crate::types::*;

type Context = Rc<RefCell<Vec<u8>>>;

const LOADPRESTATEROOT_FUNC_INDEX: usize = 0;
const BLOCKDATASIZE_FUNC_INDEX: usize = 1;
const BLOCKDATACOPY_FUNC_INDEX: usize = 2;
const SAVEPOSTSTATEROOT_FUNC_INDEX: usize = 3;
const PUSHNEWDEPOSIT_FUNC_INDEX: usize = 4;
const EXECCODE_FUNC_INDEX: usize = 5;
const GETCONTEXT_FUNC_INDEX: usize = 6;
const RETURNDATASIZE_FUNC_INDEX: usize = 7;
const RETURNDATACOPY_FUNC_INDEX: usize = 8;
const SAVERETURNDATA_FUNC_INDEX: usize = 9;
const CONTEXTDATASIZE_FUNC_INDEX: usize = 10;
const CONTEXTDATACOPY_FUNC_INDEX: usize = 11;
const PRINT_FUNC_INDEX: usize = 100;

struct Runtime<'a> {
    pub memory: Option<MemoryRef>,
    pub context: Context,
    pre_state: &'a Bytes32,
    block_data: &'a ShardBlockBody,
    return_data: Vec<u8>,
    post_state: Bytes32,
}

impl<'a> Runtime<'a> {
    fn new(
        pre_state: &'a Bytes32,
        block_data: &'a ShardBlockBody,
        context: Context,
    ) -> Runtime<'a> {
        Runtime {
            memory: Some(MemoryInstance::alloc(Pages(1), Some(Pages(1))).unwrap()),
            context: context,
            pre_state: pre_state,
            block_data: block_data,
            return_data: vec![],
            post_state: Bytes32::default(),
        }
    }

    fn get_post_state(&self) -> Bytes32 {
        self.post_state
    }

    fn get_return_data(&self) -> Vec<u8> {
        self.return_data.clone()
    }
}

impl<'a> Externals for Runtime<'a> {
    fn invoke_index(
        &mut self,
        index: usize,
        args: RuntimeArgs,
    ) -> Result<Option<RuntimeValue>, Trap> {
        match index {
            LOADPRESTATEROOT_FUNC_INDEX => {
                let ptr: u32 = args.nth(0);
                println!("loadprestateroot to {}", ptr);

                // TODO: add checks for out of bounds access
                let memory = self.memory.as_ref().expect("expects memory");
                memory.set(ptr, &self.pre_state.bytes).unwrap();

                Ok(None)
            }
            SAVEPOSTSTATEROOT_FUNC_INDEX => {
                let ptr: u32 = args.nth(0);
                println!("savepoststateroot from {}", ptr);

                // TODO: add checks for out of bounds access
                let memory = self.memory.as_ref().expect("expects memory");
                memory.get_into(ptr, &mut self.post_state.bytes).unwrap();

                Ok(None)
            }
            BLOCKDATASIZE_FUNC_INDEX => {
                let ret: i32 = self.block_data.data.len() as i32;
                println!("blockdatasize {}", ret);
                Ok(Some(ret.into()))
            }
            BLOCKDATACOPY_FUNC_INDEX => {
                let ptr: u32 = args.nth(0);
                let offset: u32 = args.nth(1);
                let length: u32 = args.nth(2);
                println!(
                    "blockdatacopy to {} from {} for {} bytes",
                    ptr, offset, length
                );

                // TODO: add overflow check
                let offset = offset as usize;
                let length = length as usize;

                // TODO: add checks for out of bounds access
                let memory = self.memory.as_ref().expect("expects memory");
                memory
                    .set(ptr, &self.block_data.data[offset..length])
                    .unwrap();

                Ok(None)
            }
            RETURNDATASIZE_FUNC_INDEX => {
                let ret: i32 = self.return_data.len() as i32;
                println!("returndatasize {}", ret);
                Ok(Some(ret.into()))
            }
            RETURNDATACOPY_FUNC_INDEX => {
                let ptr: u32 = args.nth(0);
                let offset: u32 = args.nth(1);
                let length: u32 = args.nth(2);
                println!(
                    "returndatacopy to {} from {} for {} bytes",
                    ptr, offset, length
                );

                // TODO: add overflow check
                let offset = offset as usize;
                let length = length as usize;

                // TODO: add checks for out of bounds access
                let memory = self.memory.as_ref().expect("expects memory");
                memory.set(ptr, &self.return_data[offset..length]).unwrap();

                Ok(None)
            }
            CONTEXTDATASIZE_FUNC_INDEX => {
                let ret: i32 = self.context.borrow().len() as i32;
                println!("contextdatasize {}", ret);
                Ok(Some(ret.into()))
            }
            CONTEXTDATACOPY_FUNC_INDEX => {
                let ptr: u32 = args.nth(0);
                let offset: u32 = args.nth(1);
                let length: u32 = args.nth(2);
                println!(
                    "contextdatacopy to {} from {} for {} bytes",
                    ptr, offset, length
                );

                // TODO: add overflow check
                let offset = offset as usize;
                let length = length as usize;

                // TODO: add checks for out of bounds access
                let memory = self.memory.as_ref().expect("expects memory");
                memory
                    .set(ptr, &self.context.borrow()[offset..length])
                    .unwrap();

                Ok(None)
            }
            SAVERETURNDATA_FUNC_INDEX => {
                let ptr: u32 = args.nth(0);
                let length: u32 = args.nth(1);

                println!("savereturndata from {}", ptr);

                let memory = self.memory.as_ref().expect("expects memory");
                let return_data = memory.get(ptr, length as usize).unwrap();

                self.return_data = return_data;

                Ok(None)
            }
            PUSHNEWDEPOSIT_FUNC_INDEX => unimplemented!(),
            EXECCODE_FUNC_INDEX => {
                let code_ptr: u32 = args.nth(0);
                let code_length: u32 = args.nth(1);
                let calldata_ptr: u32 = args.nth(2);
                let calldata_length: u32 = args.nth(3);
                let ctx_ptr: u32 = args.nth(4);
                let ctx_length: u32 = args.nth(5);

                println!("EEI execute_code at {} for {} bytes", code_ptr, code_length);

                let memory = self.memory.as_ref().expect("expects memory");
                let code = memory.get(code_ptr, code_length as usize).unwrap();
                let calldata = memory.get(calldata_ptr, calldata_length as usize).unwrap();
                let ctx = memory.get(ctx_ptr, ctx_length as usize).unwrap();

                *self.context.borrow_mut() = ctx;

                let (_, _, return_data) = execute_code(
                    &code,
                    self.pre_state,
                    &ShardBlockBody { data: calldata },
                    self.context.clone(),
                );

                self.return_data = return_data;

                Ok(None)
            }
            GETCONTEXT_FUNC_INDEX => {
                let ptr: u32 = args.nth(0);
                let offset: u32 = args.nth(1);
                let length: u32 = args.nth(2);

                println!("getcontext to {} from {} for {} bytes", ptr, offset, length);

                let offset = offset as usize;
                let length = length as usize;
                let memory = self.memory.as_ref().expect("expects memory");

                memory.set(ptr, &self.context.borrow()[offset..length]);

                Ok(None)
            }
            PRINT_FUNC_INDEX => {
                let ptr: u32 = args.nth(0);
                let length: u32 = args.nth(1);

                let memory = self.memory.as_ref().expect("expects memory");
                let obj = memory.get(ptr, length as usize).unwrap();

                println!("{:?}", obj);

                Ok(None)
            }
            index => panic!("unknown function index: {}", index),
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
            "eth2_returnDataSize" => FuncInstance::alloc_host(
                Signature::new(&[][..], Some(ValueType::I32)),
                RETURNDATASIZE_FUNC_INDEX,
            ),
            "eth2_returnDataCopy" => FuncInstance::alloc_host(
                Signature::new(&[ValueType::I32, ValueType::I32, ValueType::I32][..], None),
                RETURNDATACOPY_FUNC_INDEX,
            ),
            "eth2_saveReturnData" => FuncInstance::alloc_host(
                Signature::new(&[ValueType::I32, ValueType::I32][..], None),
                SAVERETURNDATA_FUNC_INDEX,
            ),
            "eth2_contextDataSize" => FuncInstance::alloc_host(
                Signature::new(&[][..], Some(ValueType::I32)),
                CONTEXTDATASIZE_FUNC_INDEX,
            ),
            "eth2_contextDataCopy" => FuncInstance::alloc_host(
                Signature::new(&[ValueType::I32, ValueType::I32, ValueType::I32][..], None),
                CONTEXTDATACOPY_FUNC_INDEX,
            ),
            "eth2_savePostStateRoot" => FuncInstance::alloc_host(
                Signature::new(&[ValueType::I32][..], None),
                SAVEPOSTSTATEROOT_FUNC_INDEX,
            ),
            "eth2_pushNewDeposit" => FuncInstance::alloc_host(
                Signature::new(&[ValueType::I32][..], None),
                PUSHNEWDEPOSIT_FUNC_INDEX,
            ),
            "eth2_execCode" => FuncInstance::alloc_host(
                Signature::new(
                    &[
                        ValueType::I32,
                        ValueType::I32,
                        ValueType::I32,
                        ValueType::I32,
                        ValueType::I32,
                        ValueType::I32,
                    ][..],
                    None,
                ),
                EXECCODE_FUNC_INDEX,
            ),
            "print" => FuncInstance::alloc_host(
                Signature::new(&[ValueType::I32, ValueType::I32][..], None),
                PRINT_FUNC_INDEX,
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
    context: Context,
) -> (Bytes32, Vec<Deposit>, Vec<u8>) {
    let module = Module::from_buffer(&code).unwrap();
    let mut imports = ImportsBuilder::new();
    // FIXME: use eth2
    imports.push_resolver("env", &RuntimeModuleImportResolver);

    let instance = ModuleInstance::new(&module, &imports)
        .unwrap()
        .assert_no_start();

    let mut runtime = Runtime::new(pre_state, block_data, context.clone());

    let internal_mem = instance
        .export_by_name("memory")
        .expect("Module expected to have 'memory' export")
        .as_memory()
        .cloned()
        .expect("'memory' export should be a memory");

    runtime.memory = Some(internal_mem);

    let result = instance
        .invoke_export("main", &[], &mut runtime)
        .expect("Executed 'main'");

    println!("Result: {:?}", result);
    println!("Execution finished");

    (
        runtime.get_post_state(),
        vec![Deposit {}],
        runtime.get_return_data(),
    )
}

pub fn process_shard_block(
    state: &mut ShardState,
    beacon_state: &BeaconState,
    block: Option<ShardBlock>,
) {
    // TODO: implement state root handling

    if let Some(block) = block {
        // The execution environment identifier
        let env = block.env as usize; // FIXME: usize can be 32-bit
        let code = &beacon_state.execution_scripts[env].code;

        // Set post states to empty for any holes
        // for x in 0..env {
        //     state.exec_env_states.push(ZERO_HASH)
        // }

        let context: Context = Default::default();
        let pre_state = &state.exec_env_states[env];
        let (post_state, deposits, _) = execute_code(code, pre_state, &block.data, context.clone());

        state.exec_env_states[env] = post_state
    }

    // TODO: implement state + deposit root handling

    // println!("Post-execution: {:#?}", state)
}

fn load_file(filename: &str) -> Vec<u8> {
    use std::io::prelude::*;
    let mut file = File::open(filename).unwrap();
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).unwrap();
    buf
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
enum TestDataValue {
    Ssz(String),
    Object(serde_yaml::Value),
}

impl TestDataValue {
    fn to_bytes(&self) -> Vec<u8> {
        match self {
            TestDataValue::Ssz(s) => s.from_hex().unwrap(),
            TestDataValue::Object(o) => to_ssz(serde_yaml::to_vec(&o).unwrap()),
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct TestBeaconState {
    execution_scripts: Vec<String>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct TestShardBlock {
    env: u64,
    data: TestDataValue,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct TestShardState {
    exec_env_states: Vec<TestDataValue>,
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
                data: input.data.to_bytes(),
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
                    let hash: Vec<u8> = match x {
                        TestDataValue::Ssz(_) => x.to_bytes(),
                        TestDataValue::Object(_) => Keccak256::digest(&x.to_bytes()[..])[..].into(),
                    };
                    assert!(hash.len() == 32);
                    let mut ret = Bytes32::default();
                    ret.bytes.copy_from_slice(&hash[..]);
                    ret
                })
                .collect(),
            slot: 0,
            parent_block: ShardBlockHeader {},
        }
    }
}

fn process_yaml_test(filename: &str) {
    println!("Process yaml!");
    let content = load_file(&filename);
    let test_file: TestFile = serde_yaml::from_slice::<TestFile>(&content[..]).unwrap();
    println!("{:#?}", test_file);

    let beacon_state: BeaconState = test_file.beacon_state.into();
    let pre_state: ShardState = test_file.shard_pre_state.into();
    let post_state: ShardState = test_file.shard_post_state.into();

    let mut shard_state = pre_state;
    for block in test_file.shard_blocks {
        process_shard_block(&mut shard_state, &beacon_state, Some(block.into()))
    }
    println!("{:#?}", shard_state);
    assert_eq!(shard_state, post_state);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    process_yaml_test(if args.len() != 2 {
        "test.yaml"
    } else {
        &args[1]
    });
}
