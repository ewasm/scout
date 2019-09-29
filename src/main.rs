extern crate rustc_hex;
extern crate wasmi;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate ssz;
#[macro_use]
extern crate ssz_derive;

use primitive_types::U256;
use rustc_hex::{FromHex, ToHex};
use serde::{Deserialize, Serialize};
use ssz::{Decode, Encode};
use std::convert::{TryFrom, TryInto};
use std::env;
use std::error::Error;
use std::fmt;
use wasmi::{
    Error as InterpreterError, Externals, FuncInstance, FuncRef, ImportsBuilder, MemoryRef, Module,
    ModuleImportResolver, ModuleInstance, NopExternals, RuntimeArgs, RuntimeValue, Signature, Trap,
    TrapKind, ValueType,
};

mod types;
use crate::types::*;

#[derive(Debug)]
pub struct ScoutError(String);

impl From<std::io::Error> for ScoutError {
    fn from(error: std::io::Error) -> Self {
        ScoutError {
            0: error.description().to_string(),
        }
    }
}

impl From<rustc_hex::FromHexError> for ScoutError {
    fn from(error: rustc_hex::FromHexError) -> Self {
        ScoutError {
            0: error.description().to_string(),
        }
    }
}

impl From<wasmi::Error> for ScoutError {
    fn from(error: wasmi::Error) -> Self {
        ScoutError {
            0: error.description().to_string(),
        }
    }
}

impl From<wasmi::Trap> for ScoutError {
    fn from(error: wasmi::Trap) -> Self {
        ScoutError {
            0: error.description().to_string(),
        }
    }
}

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
const BIGNUM_ADD256_FUNC: usize = 10;
const BIGNUM_SUB256_FUNC: usize = 11;

// TODO: move elsehwere?
type DepositBlob = Vec<u8>;

struct Runtime<'a> {
    ticks_left: u32,
    memory: Option<MemoryRef>,
    pre_state: &'a Bytes32,
    block_data: &'a ShardBlockBody,
    post_state: Bytes32,
    deposits: Vec<DepositBlob>,
}

impl<'a> Runtime<'a> {
    fn new(
        pre_state: &'a Bytes32,
        block_data: &'a ShardBlockBody,
        memory: MemoryRef,
    ) -> Runtime<'a> {
        Runtime {
            ticks_left: 10_000_000, // FIXME: make this configurable
            memory: Some(memory),
            pre_state: pre_state,
            block_data: block_data,
            post_state: Bytes32::default(),
            deposits: vec![],
        }
    }

    fn get_post_state(&self) -> Bytes32 {
        self.post_state
    }

    fn get_deposits(&self) -> Vec<DepositBlob> {
        // TODO: avoid cloning here
        self.deposits.clone()
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
            PUSHNEWDEPOSIT_FUNC_INDEX => {
                let ptr: u32 = args.nth(0);
                let length: u32 = args.nth(1);
                info!("pushnewdeposit from {} for {} bytes", ptr, length);

                let memory = self.memory.as_ref().expect("expects memory");
                let tmp = memory
                    .get(ptr, length as usize)
                    .expect("expects reading from memory to succeed");
                debug!("deposit: {}", tmp.to_hex());
                self.deposits.push(tmp);

                Ok(None)
            }
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
            BIGNUM_ADD256_FUNC => {
                let a_ptr: u32 = args.nth(0);
                let b_ptr: u32 = args.nth(1);
                let c_ptr: u32 = args.nth(2);

                let mut a_raw = [0u8; 32];
                let mut b_raw = [0u8; 32];
                let mut c_raw = [0u8; 32];

                let memory = self.memory.as_ref().expect("expects memory object");
                memory
                    .get_into(a_ptr, &mut a_raw)
                    .expect("expects reading from memory to succeed");
                memory
                    .get_into(b_ptr, &mut b_raw)
                    .expect("expects reading from memory to succeed");

                let a = U256::from_big_endian(&a_raw);
                let b = U256::from_big_endian(&b_raw);
                let c = a.checked_add(b).expect("expects non-overflowing addition");
                c.to_big_endian(&mut c_raw);

                memory
                    .set(c_ptr, &c_raw)
                    .expect("expects writing to memory to succeed");

                Ok(None)
            }
            BIGNUM_SUB256_FUNC => {
                let a_ptr: u32 = args.nth(0);
                let b_ptr: u32 = args.nth(1);
                let c_ptr: u32 = args.nth(2);

                let mut a_raw = [0u8; 32];
                let mut b_raw = [0u8; 32];
                let mut c_raw = [0u8; 32];

                let memory = self.memory.as_ref().expect("expects memory object");
                memory
                    .get_into(a_ptr, &mut a_raw)
                    .expect("expects reading from memory to succeed");
                memory
                    .get_into(b_ptr, &mut b_raw)
                    .expect("expects reading from memory to succeed");

                let a = U256::from_big_endian(&a_raw);
                let b = U256::from_big_endian(&b_raw);
                let c = a
                    .checked_sub(b)
                    .expect("expects non-overflowing subtraction");
                c.to_big_endian(&mut c_raw);

                memory
                    .set(c_ptr, &c_raw)
                    .expect("expects writing to memory to succeed");

                Ok(None)
            }
            _ => panic!("unknown function index"),
        }
    }
}

// TODO: remove this and rely on Eth2ImportResolver and DebugImportResolver
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
            "bignum_add256" => FuncInstance::alloc_host(
                Signature::new(&[ValueType::I32, ValueType::I32, ValueType::I32][..], None),
                BIGNUM_ADD256_FUNC,
            ),
            "bignum_sub256" => FuncInstance::alloc_host(
                Signature::new(&[ValueType::I32, ValueType::I32, ValueType::I32][..], None),
                BIGNUM_SUB256_FUNC,
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

struct Eth2ImportResolver;

impl<'a> ModuleImportResolver for Eth2ImportResolver {
    fn resolve_func(
        &self,
        field_name: &str,
        _signature: &Signature,
    ) -> Result<FuncRef, InterpreterError> {
        let func_ref = match field_name {
            "useTicks" => FuncInstance::alloc_host(
                Signature::new(&[ValueType::I32][..], None),
                USETICKS_FUNC_INDEX,
            ),
            "loadPreStateRoot" => FuncInstance::alloc_host(
                Signature::new(&[ValueType::I32][..], None),
                LOADPRESTATEROOT_FUNC_INDEX,
            ),
            "blockDataSize" => FuncInstance::alloc_host(
                Signature::new(&[][..], Some(ValueType::I32)),
                BLOCKDATASIZE_FUNC_INDEX,
            ),
            "blockDataCopy" => FuncInstance::alloc_host(
                Signature::new(&[ValueType::I32, ValueType::I32, ValueType::I32][..], None),
                BLOCKDATACOPY_FUNC_INDEX,
            ),
            "savePostStateRoot" => FuncInstance::alloc_host(
                Signature::new(&[ValueType::I32][..], None),
                SAVEPOSTSTATEROOT_FUNC_INDEX,
            ),
            "pushNewDeposit" => FuncInstance::alloc_host(
                Signature::new(&[ValueType::I32, ValueType::I32][..], None),
                PUSHNEWDEPOSIT_FUNC_INDEX,
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

struct BignumImportResolver;

impl<'a> ModuleImportResolver for BignumImportResolver {
    fn resolve_func(
        &self,
        field_name: &str,
        _signature: &Signature,
    ) -> Result<FuncRef, InterpreterError> {
        let func_ref = match field_name {
            "add256" => FuncInstance::alloc_host(
                Signature::new(&[ValueType::I32, ValueType::I32, ValueType::I32][..], None),
                BIGNUM_ADD256_FUNC,
            ),
            "sub256" => FuncInstance::alloc_host(
                Signature::new(&[ValueType::I32, ValueType::I32, ValueType::I32][..], None),
                BIGNUM_SUB256_FUNC,
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

struct DebugImportResolver;

impl<'a> ModuleImportResolver for DebugImportResolver {
    fn resolve_func(
        &self,
        field_name: &str,
        _signature: &Signature,
    ) -> Result<FuncRef, InterpreterError> {
        let func_ref = match field_name {
            "print32" => FuncInstance::alloc_host(
                Signature::new(&[ValueType::I32][..], None),
                DEBUG_PRINT32_FUNC,
            ),
            "print64" => FuncInstance::alloc_host(
                Signature::new(&[ValueType::I64][..], None),
                DEBUG_PRINT64_FUNC,
            ),
            "printMem" => FuncInstance::alloc_host(
                Signature::new(&[ValueType::I32, ValueType::I32][..], None),
                DEBUG_PRINTMEM_FUNC,
            ),
            "printMemHex" => FuncInstance::alloc_host(
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

#[derive(Default, PartialEq, Clone, Debug, Ssz)]
pub struct Hash([u8; 32]);

#[derive(Clone, Ssz)]
pub struct BLSPubKey([u8; 48]);

impl PartialEq for BLSPubKey {
    fn eq(&self, other: &Self) -> bool {
        self.0[..] == other.0[..]
    }
}

impl Default for BLSPubKey {
    fn default() -> Self {
        BLSPubKey { 0: [0u8; 48] }
    }
}

impl fmt::Debug for BLSPubKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.to_hex())
    }
}

#[derive(Clone, Ssz)]
pub struct BLSSignature([u8; 96]);

impl PartialEq for BLSSignature {
    fn eq(&self, other: &Self) -> bool {
        self.0[..] == other.0[..]
    }
}

impl Default for BLSSignature {
    fn default() -> Self {
        BLSSignature { 0: [0u8; 96] }
    }
}

impl fmt::Debug for BLSSignature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.to_hex())
    }
}

/// These are Phase 0 structures.
/// https://github.com/ethereum/eth2.0-specs/blob/dev/specs/core/0_beacon-chain.md
/// basically this is a little-endian tightly packed representation of those fields.
#[derive(Default, PartialEq, Clone, Debug, Ssz)]
pub struct Deposit {
    pubkey: BLSPubKey,
    withdrawal_credentials: Hash,
    amount: u64,
    signature: BLSSignature,
}

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

impl fmt::Display for ShardBlockBody {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.data.to_hex())
    }
}

impl fmt::Display for ShardBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Shard block for environment {} with data {}",
            self.env, self.data
        )
    }
}

impl fmt::Display for ShardState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let states: Vec<String> = self
            .exec_env_states
            .iter()
            .map(|x| x.bytes.to_hex())
            .collect();
        write!(
            f,
            "Shard slot {} with environment states: {:?}",
            self.slot, states
        )
    }
}

pub fn execute_code(
    code: &[u8],
    pre_state: &Bytes32,
    block_data: &ShardBlockBody,
) -> Result<(Bytes32, Vec<DepositBlob>), ScoutError> {
    debug!(
        "Executing codesize({}) and data: {}",
        code.len(),
        block_data
    );

    let module = Module::from_buffer(&code)?;
    let mut imports = ImportsBuilder::new();
    // TODO: remove this and rely on Eth2ImportResolver and DebugImportResolver
    imports.push_resolver("env", &RuntimeModuleImportResolver);
    imports.push_resolver("eth2", &Eth2ImportResolver);
    imports.push_resolver("bignum", &BignumImportResolver);
    imports.push_resolver("debug", &DebugImportResolver);

    let instance = ModuleInstance::new(&module, &imports)?.run_start(&mut NopExternals)?;

    // FIXME: pass through errors here and not use .expect()
    let internal_mem = instance
        .export_by_name("memory")
        .expect("Module expected to have 'memory' export")
        .as_memory()
        .cloned()
        .expect("'memory' export should be a memory");

    let mut runtime = Runtime::new(pre_state, block_data, internal_mem);

    let result = instance.invoke_export("main", &[], &mut runtime)?;

    info!("Result: {:?}", result);
    info!("Execution finished");

    Ok((runtime.get_post_state(), runtime.get_deposits()))
}

pub fn process_shard_block(
    state: &mut ShardState,
    beacon_state: &BeaconState,
    block: Option<ShardBlock>,
) -> Result<Vec<Deposit>, ScoutError> {
    // println!("Beacon state: {:#?}", beacon_state);

    info!("Pre-execution: {}", state);

    // TODO: implement state root handling

    let deposit_receipts = if let Some(block) = block {
        info!("Executing block: {}", block);

        // The execution environment identifier
        let env = block.env as usize; // FIXME: usize can be 32-bit
        let code = &beacon_state.execution_scripts[env].code;

        // Set post states to empty for any holes
        // for x in 0..env {
        //     state.exec_env_states.push(ZERO_HASH)
        // }
        let pre_state = &state.exec_env_states[env];
        let (post_state, deposits) = execute_code(code, pre_state, &block.data)?;
        state.exec_env_states[env] = post_state;

        // Decode deposits.
        deposits
            .into_iter()
            .map(|deposit| {
                let mut deposit: &[u8] = &deposit;
                // FIXME: remove the expect from here
                Deposit::decode(&mut deposit).expect("valid SSZ decodable deposit")
            })
            .collect()
    } else {
        Vec::new()
    };

    // TODO: implement state + deposit root handling

    info!("Post-execution deposit receipts: {:?}", deposit_receipts);
    info!("Post-execution: {}", state);

    Ok(deposit_receipts)
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
struct TestDeposit {
    pubkey: String,
    withdrawal_credentials: String,
    amount: u64,
    signature: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct TestFile {
    beacon_state: TestBeaconState,
    shard_blocks: Vec<TestShardBlock>,
    shard_pre_state: TestShardState,
    shard_post_state: TestShardState,
    deposit_receipts: Vec<TestDeposit>,
}

fn hex_to_slice(input: &str, output: &mut [u8]) -> Result<(), ScoutError> {
    let tmp = input.from_hex()?;
    if tmp.len() != output.len() {
        return Err(ScoutError("Length mismatch from hex input".to_string()));
    }
    output.copy_from_slice(&tmp[..]);
    Ok(())
}

impl TryFrom<&String> for Bytes32 {
    type Error = ScoutError;
    fn try_from(input: &String) -> Result<Self, Self::Error> {
        let mut ret = Bytes32::default();
        hex_to_slice(input, &mut ret.bytes)?;
        Ok(ret)
    }
}

impl TryFrom<String> for Hash {
    type Error = ScoutError;
    fn try_from(input: String) -> Result<Self, Self::Error> {
        let mut ret = Hash::default();
        hex_to_slice(&input, &mut ret.0)?;
        Ok(ret)
    }
}

impl TryFrom<String> for BLSPubKey {
    type Error = ScoutError;
    fn try_from(input: String) -> Result<Self, Self::Error> {
        let mut ret = BLSPubKey::default();
        hex_to_slice(&input, &mut ret.0)?;
        Ok(ret)
    }
}

impl TryFrom<String> for BLSSignature {
    type Error = ScoutError;
    fn try_from(input: String) -> Result<Self, Self::Error> {
        let mut ret = BLSSignature::default();
        hex_to_slice(&input, &mut ret.0)?;
        Ok(ret)
    }
}

impl TryFrom<TestBeaconState> for BeaconState {
    type Error = ScoutError;
    fn try_from(input: TestBeaconState) -> Result<Self, Self::Error> {
        let scripts: Result<Vec<ExecutionScript>, ScoutError> = input
            .execution_scripts
            .iter()
            .map(|filename| {
                Ok(ExecutionScript {
                    code: std::fs::read(filename)?,
                })
            })
            .collect();
        Ok(BeaconState {
            execution_scripts: scripts?,
        })
    }
}

impl TryFrom<TestShardBlock> for ShardBlock {
    type Error = ScoutError;
    fn try_from(input: TestShardBlock) -> Result<Self, Self::Error> {
        Ok(ShardBlock {
            env: input.env,
            data: ShardBlockBody {
                data: input.data.from_hex()?,
            },
        })
    }
}

impl TryFrom<TestShardState> for ShardState {
    type Error = ScoutError;
    fn try_from(input: TestShardState) -> Result<Self, Self::Error> {
        let states: Result<Vec<Bytes32>, ScoutError> = input
            .exec_env_states
            .iter()
            .map(|state| state.try_into())
            .collect();

        Ok(ShardState {
            exec_env_states: states?,
            slot: 0,
            parent_block: ShardBlockHeader {},
        })
    }
}

impl TryFrom<TestDeposit> for Deposit {
    type Error = ScoutError;
    fn try_from(input: TestDeposit) -> Result<Self, Self::Error> {
        Ok(Deposit {
            pubkey: input.pubkey.try_into()?,
            withdrawal_credentials: input.withdrawal_credentials.try_into()?,
            amount: input.amount,
            signature: input.signature.try_into()?,
        })
    }
}

fn process_yaml_test(filename: &str) {
    info!("Processing {}...", filename);
    let content = std::fs::read(&filename).expect("to load file");
    let test_file: TestFile =
        serde_yaml::from_slice::<TestFile>(&content[..]).expect("expected valid yaml");
    debug!("{:#?}", test_file);

    let beacon_state: BeaconState = test_file
        .beacon_state
        .try_into()
        .expect("valid beacon_state definition");
    let pre_state: ShardState = test_file
        .shard_pre_state
        .try_into()
        .expect("valid pre_state befinition");
    let post_state: ShardState = test_file
        .shard_post_state
        .try_into()
        .expect("valid post_state definition");
    let expected_deposit_receipts: Vec<Deposit> = test_file
        .deposit_receipts
        .into_iter()
        .map(|deposit| deposit.try_into().expect("valid deposit"))
        .collect();

    let mut shard_state = pre_state;
    let mut deposit_receipts = Vec::new();
    for block in test_file.shard_blocks {
        deposit_receipts.append(
            process_shard_block(
                &mut shard_state,
                &beacon_state,
                Some(block.try_into().expect("valid block")),
            )
            .expect("processing shard block to succeed")
            .as_mut(),
        );
    }

    if expected_deposit_receipts
        .iter()
        .all(|deposit| deposit_receipts.contains(deposit))
    {
        println!("Matching deposit receipts.")
    } else {
        println!("Expected deposit receipts: {:?}", expected_deposit_receipts);
        println!("Got deposit receipts: {:?}", deposit_receipts);
        std::process::exit(1);
    }

    debug!("{}", shard_state);
    if shard_state != post_state {
        println!("Expected state: {}", post_state);
        println!("Got state: {}", shard_state);
        std::process::exit(1);
    } else {
        println!("Matching state.");
    }
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
