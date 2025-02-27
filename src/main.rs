use std::{fs::File, io::BufReader, num::NonZeroUsize, sync::Arc};

use alloy_primitives::{keccak256, Address, TxKind, U256};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use hashbrown::HashMap;
use pevm::{
    chain::PevmEthereum, BlockHashes, BuildSuffixHasher, Bytecodes, EvmAccount, InMemoryStorage,
    Pevm,
};
use revm::primitives::{
    Account, AccountInfo, BlockEnv, Bytecode, EvmStorage, EvmStorageSlot, SpecId, TxEnv,
};
use revme::cmd::statetest::models::TestSuite;

/// Benchmark the execution of a single input json test file
fn benchmark_input_json(c: &mut Criterion, input_json_file_name: &str, num_txs: usize) {
    let mut pevm = Pevm::default();
    let chain = PevmEthereum::mainnet();
    let concurrency_level = NonZeroUsize::new(8).unwrap();

    let data_dir = std::path::PathBuf::from("data");
    let input_json = data_dir.join(input_json_file_name);

    let suite: TestSuite =
        serde_json::from_reader(BufReader::new(File::open(input_json).unwrap())).unwrap();

    let test = suite.0;
    let test = test.first_key_value().unwrap().1;

    let mut bytecodes: Bytecodes = Bytecodes::default();
    let mut accounts: HashMap<Address, EvmAccount, BuildSuffixHasher> = HashMap::default();
    test.pre.iter().for_each(|(address, account)| {
        let code_hash = keccak256(&account.code);
        let code = match account.code.len() > 0 {
            true => Some(Bytecode::new_raw(account.code.clone())),
            false => None,
        };

        let storage: EvmStorage = account
            .storage
            .iter()
            .map(|(k, v)| {
                (
                    *k,
                    EvmStorageSlot {
                        original_value: *v,
                        present_value: *v,
                        is_cold: true,
                    },
                )
            })
            .collect();

        let account = Account {
            info: AccountInfo {
                nonce: account.nonce,
                balance: account.balance,
                code,
                code_hash,
            },
            storage,
            ..Default::default()
        };
        accounts.insert(*address, account.into());
    });

    let sender_account = accounts
        .get(&test.transaction.sender.unwrap())
        .unwrap()
        .clone();

    let contract_account = accounts.get(&test.transaction.to.unwrap()).unwrap().clone();

    // iter over accounts and get the bytecodes
    accounts.iter().for_each(|(_address, account)| {
        if let Some(ref bytecode) = account.code.clone() {
            bytecodes.insert(account.code_hash.unwrap(), bytecode.clone());
        }
    });

    let bytecodes = Arc::new(bytecodes);

    let block_hashes: BlockHashes = BlockHashes::default();
    let block_hashes = Arc::new(block_hashes);

    let mut block_env = BlockEnv::default();
    let spec_id = SpecId::SHANGHAI;

    block_env.basefee = test.env.current_base_fee.unwrap();
    block_env.difficulty = test.env.current_difficulty;
    block_env.timestamp = test.env.current_timestamp;
    block_env.coinbase = test.env.current_coinbase;
    block_env.gas_limit = test.env.current_gas_limit;
    block_env.number = test.env.current_number;

    let mut tx = TxEnv {
        caller: test.transaction.sender.unwrap(),
        ..Default::default()
    };

    tx.gas_price = test.transaction.gas_price.unwrap();
    tx.gas_priority_fee = test.transaction.max_priority_fee_per_gas;
    tx.blob_hashes = test.transaction.blob_versioned_hashes.clone();
    tx.max_fee_per_blob_gas = test.transaction.max_fee_per_blob_gas;
    tx.data = test.transaction.data[0].clone();
    tx.gas_limit = test.transaction.gas_limit[0].try_into().unwrap();
    let value = &test.transaction.value[0];
    tx.value = if let Some(stripped) = value.strip_prefix("0x") {
        U256::from_str_radix(stripped, 16).unwrap()
    } else {
        U256::from_str_radix(value, 16).unwrap()
    };
    let initial_nonce = u64::try_from(test.transaction.nonce).unwrap();
    tx.nonce = Some(initial_nonce);
    let to = match test.transaction.to {
        Some(add) => TxKind::Call(add),
        None => TxKind::Create,
    };
    tx.transact_to = to;

    let mut txs = vec![];
    for _ in 0..num_txs {
        let mut tx = tx.clone();
        let rand_address = Address::new(rand::random());
        accounts.insert(rand_address, sender_account.clone());

        // todo need to modify the storage in the contract to make the sender have proper states

        tx.caller = rand_address;
        txs.push(tx);
    }

    let storage = InMemoryStorage::new(accounts, Arc::clone(&bytecodes), Arc::clone(&block_hashes));

    let id = format!(
        "{} ({} transactions per iteration)",
        input_json_file_name, num_txs
    );
    c.bench_function(&id, |b| {
        b.iter(|| {
            pevm.execute_revm_parallel(
                black_box(&chain),
                black_box(&storage),
                black_box(spec_id),
                black_box(block_env.clone()),
                black_box(txs.clone()),
                black_box(concurrency_level),
            )
            .unwrap();
        })
    });
}

struct TestCase {
    filename: String,
    txs_nums: Vec<usize>,
}

fn main_benchmark(c: &mut Criterion) {
    let test_cases = vec![
        TestCase {
            filename: "erc20_transfer_basic_usdt.json".into(),
            txs_nums: vec![1, 10, 100, 200, 2000],
        },
        TestCase {
            filename: "erc20_transfer_proxy_usdc.json".into(),
            txs_nums: vec![1, 10, 100, 200, 2000],
        },
        TestCase {
            filename: "uniswap2_single.json".into(),
            txs_nums: vec![1, 10, 100],
        },
        TestCase {
            filename: "uniswap2_multiple.json".into(),
            txs_nums: vec![1],
        },
        TestCase {
            filename: "uniswap3_single.json".into(),
            txs_nums: vec![1],
        },
    ];

    // test each case
    for test_case in test_cases {
        for txs_num in test_case.txs_nums {
            benchmark_input_json(c, &test_case.filename, txs_num);
        }
    }
}

criterion_group!(benches, main_benchmark);
criterion_main!(benches);
