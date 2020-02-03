const assert = require('assert')
const { promisify } = require('util')
const BN = require('bn.js')
const Trie = require('merkle-patricia-tree')
const Account = require('ethereumjs-account').default
const StateManager = require('ethereumjs-vm/dist/state/stateManager').default
const PStateManager = require('ethereumjs-vm/dist/state/promisified').default
const { keccak256, ecsign, stripZeros } = require('ethereumjs-util')
const { encode } = require('rlp')
const Wallet = require('ethereumjs-wallet')
const yaml = require('js-yaml')
const fs = require('fs')

const prove = promisify(Trie.prove)
const verifyProof = promisify(Trie.verifyProof)

async function main () {
  const testSuite = {
    'beacon_state': {
      'execution_scripts': [
        'target/wasm32-unknown-unknown/release/smpt.wasm'
      ],
    },
    'shard_pre_state': {
      'exec_env_states': [
      ]
    },
    'shard_blocks': [
    ],
    'shard_post_state': {
      'exec_env_states': [
      ]
    }
  }

  const rawState = new StateManager()
  const state = new PStateManager(rawState)

  // Generate random accounts
  let accounts = await generateAccounts(state, 5000)

  let root = await state.getStateRoot()
  testSuite.shard_pre_state.exec_env_states.push(root.toString('hex'))

  // Generate txes
  let [txes, proofNodes, simulationData] = await generateTxes(state, accounts, 70)

  // Serialize witnesses and tx data
  const blockData = encode([txes, proofNodes])
  console.log(`block data length: ${blockData.length}`)
  testSuite.shard_blocks.push({
    'env': 0,
    'data': blockData.toString('hex')
  })

  // Apply txes on top of trie to compute post state root
  for (tx of simulationData) {
    await transfer(state, tx)
  }

  root = await state.getStateRoot()
  testSuite.shard_post_state.exec_env_states.push(root.toString('hex'))

  const serializedTestSuite = yaml.safeDump(testSuite)
  fs.writeFileSync('smpt.yaml', serializedTestSuite)
}

async function generateTxes (state, accounts, count=50) {
  let txes = []
  let simulationData = []
  let proofNodes = {}
  const root = await state.getStateRoot()
  let totalNodes = 0
  for (let i = 0; i < count; i++) {
    const from = accounts[i].address
    const to = accounts[i + 1].address
    const value = new BN('00000000000000000000000000000000000000000000000000000000000000ff', 16)
    const nonce = new BN('0000000000000000000000000000000000000000000000000000000000000000', 16)
    simulationData.push({ from, to, value, nonce })

    const fromAccount = await state.getAccount(from)
    const fromWitness = await prove(state._wrapped._trie, keccak256(from))
    let val = await verifyProof(root, keccak256(from), fromWitness)
    assert(val.equals(fromAccount.serialize()), "valid from witness")
    for (item of fromWitness) {
      totalNodes++
      proofNodes[keccak256(item).toString('hex')] = item
    }

    const toAccount = await state.getAccount(to)
    const toWitness = await prove(state._wrapped._trie, keccak256(to))
    val = await verifyProof(root, keccak256(to), toWitness)
    assert(val.equals(toAccount.serialize()), "valid to witness")
    for (item of toWitness) {
      totalNodes++
      proofNodes[keccak256(item).toString('hex')] = item
    }

    const txRlp = encode([to, stripZeros(value.toBuffer('be', 32)), stripZeros(nonce.toBuffer('be', 32))])
    const txHash = keccak256(txRlp)
    const txSig = ecsign(txHash, accounts[i].privateKey)

    txes.push([to, stripZeros(value.toBuffer('be', 32)), stripZeros(nonce.toBuffer('be', 32)), [stripZeros(txSig.r), stripZeros(txSig.s), txSig.v]])
  }
  console.log(`Deduplicated ${totalNodes} proof nodes ${Object.values(proofNodes).length}`)
  return [txes, Object.values(proofNodes), simulationData]
}

async function transfer (state, tx) {
  let { from, to, value, nonce } = tx
  assert(value.gten(0))

  const fromAcc = await state.getAccount(from)
  const toAcc = await state.getAccount(to)

  assert(new BN(fromAcc.balance).gte(value))
  assert(new BN(fromAcc.nonce).eq(nonce))

  const newFromBalance = new BN(fromAcc.balance).sub(value)
  fromAcc.balance = newFromBalance.toBuffer()
  fromAcc.nonce = nonce.addn(1).toBuffer()
  const newToBalance = new BN(toAcc.balance).add(value)
  toAcc.balance = newToBalance.toBuffer()

  await state.putAccount(from, fromAcc)
  await state.putAccount(to, toAcc)
}

async function generateAccounts (state, count=500) {
  let accounts = []
  for (let i = 0; i < count; i++) {
    let wallet = Wallet.generate()
    let address = wallet.getAddress()
    let privateKey = wallet.getPrivateKey()
    let account = new Account()
    account.balance = new BN('ffffff', 16).toBuffer()
    accounts.push({
      address,
      privateKey,
      account
    })
    await state.putAccount(address, account)
  }
  return accounts
}

main().then(() => {}).catch((e) => console.log(e))
