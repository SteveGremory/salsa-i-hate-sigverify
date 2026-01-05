//! Benchmarks for block execution via BlockConsumer.
//!
//! Measures the performance of executing blocks of transactions using optimistic recording.

use {
    agave_reserved_account_keys::ReservedAccountKeys,
    agave_transaction_view::{
        resolved_transaction_view::ResolvedTransactionView,
        transaction_view::SanitizedTransactionView,
    },
    criterion::{
        criterion_group, criterion_main, AxisScale, BenchmarkId, Criterion, PlotConfiguration,
        Throughput,
    },
    crossbeam_channel::unbounded,
    rand::{prelude::*, thread_rng},
    rayon::prelude::*,
    solana_core::{
        banking_stage::{committer::Committer, scheduler_messages::MaxAge},
        block_stage::BlockConsumer,
    },
    solana_keypair::Keypair,
    solana_ledger::genesis_utils::{create_genesis_config_with_leader, GenesisConfigInfo},
    solana_message::Message,
    solana_native_token::LAMPORTS_PER_SOL,
    solana_poh::{
        record_channels::{record_channels, RecordReceiver},
        transaction_recorder::TransactionRecorder,
    },
    solana_runtime::{
        bank::{test_utils::deposit, Bank},
        bank_forks::BankForks,
        prioritization_fee_cache::PrioritizationFeeCache,
    },
    solana_runtime_transaction::runtime_transaction::RuntimeTransaction,
    solana_signer::Signer,
    solana_system_interface::instruction as system_instruction,
    solana_transaction::{sanitized::MessageHash, versioned::VersionedTransaction, Transaction},
    std::{sync::Arc, time::Duration},
};

const NUM_TRANSACTIONS: [usize; 5] = [1, 10, 100, 500, 1000];

fn make_config() -> (
    GenesisConfigInfo,
    Arc<Bank>,
    Arc<std::sync::RwLock<BankForks>>,
) {
    let leader_keypair = Keypair::new();
    let GenesisConfigInfo {
        mut genesis_config,
        mint_keypair,
        voting_keypair,
        ..
    } = create_genesis_config_with_leader(
        100 * LAMPORTS_PER_SOL,
        &leader_keypair.pubkey(),
        LAMPORTS_PER_SOL * 1_000_000,
    );

    // Increase ticks per slot to have more time
    genesis_config.ticks_per_slot *= 8;

    // Create bank with proper fork graph setup
    let bank = Bank::new_for_benches(&genesis_config);
    let bank_forks = BankForks::new_rw_arc(bank);
    let bank = bank_forks.read().unwrap().get(0).unwrap();

    // Set up fork graph for program cache
    bank.set_fork_graph_in_program_cache(Arc::downgrade(&bank_forks));

    (
        GenesisConfigInfo {
            genesis_config,
            mint_keypair,
            voting_keypair,
            validator_pubkey: leader_keypair.pubkey(),
        },
        bank,
        bank_forks,
    )
}

/// Convert serialized transaction bytes to RuntimeTransaction<ResolvedTransactionView>
fn to_runtime_transaction(serialized: &[u8]) -> RuntimeTransaction<ResolvedTransactionView<&[u8]>> {
    let transaction_view = SanitizedTransactionView::try_new_sanitized(serialized, true).unwrap();
    let static_runtime_tx = RuntimeTransaction::<SanitizedTransactionView<_>>::try_from(
        transaction_view,
        MessageHash::Compute,
        None,
    )
    .unwrap();
    RuntimeTransaction::<ResolvedTransactionView<_>>::try_from(
        static_runtime_tx,
        None,
        &ReservedAccountKeys::empty_key_set(),
    )
    .unwrap()
}

/// Create transactions with random account accesses.
/// Returns serialized bytes that must stay alive for the transaction views.
fn make_transactions(
    num_accounts: usize,
    num_transactions: usize,
    bank: &Bank,
    recent_blockhash: solana_hash::Hash,
) -> Vec<Vec<u8>> {
    // Create unique account keys
    let accounts: Vec<Keypair> = (0..num_accounts)
        .into_par_iter()
        .map(|_| Keypair::new())
        .collect();

    // Fund each account with enough SOL
    accounts.par_iter().for_each(|account| {
        deposit(bank, &account.pubkey(), 100 * LAMPORTS_PER_SOL).unwrap();
    });

    // Create transactions and serialize them
    (0..num_transactions)
        .into_par_iter()
        .map(|_i| {
            let mut rng = thread_rng();
            // Use 2-10 accounts per transaction
            let num_tx_accounts = rng.gen_range(2..=10.min(num_accounts));
            let selected_accounts: Vec<_> = accounts
                .choose_multiple(&mut rng, num_tx_accounts)
                .collect();

            // First account is the payer/signer
            let payer = selected_accounts[0];

            // Create transfer instructions to other accounts
            let instructions: Vec<_> = selected_accounts
                .iter()
                .skip(1)
                .map(|account| system_instruction::transfer(&payer.pubkey(), &account.pubkey(), 1))
                .collect();

            let message = Message::new(&instructions, Some(&payer.pubkey()));
            let tx = Transaction::new(&[payer], message, recent_blockhash);

            // Serialize to bytes
            bincode::serialize(&VersionedTransaction::from(tx)).unwrap()
        })
        .collect()
}

/// Create sequential transfer block (tx1 funds tx2's payer, tx2 funds tx3's payer, etc.)
/// Returns serialized bytes that must stay alive for the transaction views.
fn make_sequential_block(
    num_transactions: usize,
    bank: &Bank,
    recent_blockhash: solana_hash::Hash,
) -> Vec<Vec<u8>> {
    let keypairs: Vec<Keypair> = (0..num_transactions + 1).map(|_| Keypair::new()).collect();

    // Fund the first keypair from mint
    deposit(bank, &keypairs[0].pubkey(), 100 * LAMPORTS_PER_SOL).unwrap();

    // Create chain of transfers: keypairs[0] -> keypairs[1] -> keypairs[2] -> ...
    (0..num_transactions)
        .map(|i| {
            let from = &keypairs[i];
            let to = &keypairs[i + 1];
            let ix = system_instruction::transfer(&from.pubkey(), &to.pubkey(), LAMPORTS_PER_SOL);
            let message = Message::new(&[ix], Some(&from.pubkey()));
            let tx = Transaction::new(&[from], message, recent_blockhash);

            // Serialize to bytes
            bincode::serialize(&VersionedTransaction::from(tx)).unwrap()
        })
        .collect()
}

fn create_block_consumer(bank: &Bank) -> (BlockConsumer, TransactionRecorder, RecordReceiver) {
    let (record_sender, mut record_receiver) = record_channels(false);
    let recorder = TransactionRecorder::new(record_sender);
    record_receiver.restart(bank.bank_id());

    let (replay_vote_sender, _replay_vote_receiver) = unbounded();

    let committer = Committer::new(
        None,
        replay_vote_sender,
        Arc::new(PrioritizationFeeCache::new(0u64)),
    );

    let consumer = BlockConsumer::new(committer, recorder.clone(), None);

    (consumer, recorder, record_receiver)
}

fn bench_random_blocks(c: &mut Criterion) {
    let (genesis_config_info, bank, _bank_forks) = make_config();
    let mut group = c.benchmark_group("block_random_transactions");
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    // Pre-generate transactions for the largest block size
    let max_transactions = *NUM_TRANSACTIONS.last().unwrap();
    let num_accounts = max_transactions * 2; // Ensure enough accounts

    // Create serialized transactions (bytes must stay alive)
    let serialized_txs = make_transactions(
        num_accounts,
        max_transactions,
        &bank,
        genesis_config_info.genesis_config.hash(),
    );

    for num_transactions in NUM_TRANSACTIONS {
        group.throughput(Throughput::Elements(num_transactions as u64));

        let block_bytes = &serialized_txs[..num_transactions];
        let max_ages: Vec<MaxAge> = vec![MaxAge::MAX; num_transactions];

        group.bench_function(
            BenchmarkId::new("process_and_record_block_transactions", num_transactions),
            |b| {
                b.iter_batched(
                    || {
                        // Clear signatures before each iteration
                        bank.clear_signatures();
                        // Convert bytes to runtime transactions (zerocopy views)
                        let transactions: Vec<_> = block_bytes
                            .iter()
                            .map(|bytes| to_runtime_transaction(bytes))
                            .collect();
                        let (consumer, recorder, record_receiver) = create_block_consumer(&bank);
                        (consumer, recorder, record_receiver, transactions)
                    },
                    |(mut consumer, _recorder, _record_receiver, transactions)| {
                        let output = consumer.process_and_record_block_transactions(
                            &bank,
                            &transactions,
                            &max_ages,
                            bank.slot(),
                        );
                        // Ensure the block was processed
                        assert!(output
                            .execute_and_commit_transactions_output
                            .commit_transactions_result
                            .is_ok());
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

fn bench_sequential_blocks(c: &mut Criterion) {
    let (genesis_config_info, bank, _bank_forks) = make_config();
    let mut group = c.benchmark_group("block_sequential_transactions");
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    // Pre-generate sequential block for the largest size
    let max_transactions = *NUM_TRANSACTIONS.last().unwrap();
    let sequential_bytes = make_sequential_block(
        max_transactions,
        &bank,
        genesis_config_info.genesis_config.hash(),
    );

    for num_transactions in NUM_TRANSACTIONS {
        group.throughput(Throughput::Elements(num_transactions as u64));

        let block_bytes = &sequential_bytes[..num_transactions];
        let max_ages: Vec<MaxAge> = vec![MaxAge::MAX; num_transactions];

        group.bench_function(
            BenchmarkId::new("process_and_record_block_transactions", num_transactions),
            |b| {
                b.iter_batched(
                    || {
                        // Clear signatures before each iteration
                        bank.clear_signatures();
                        // Convert bytes to runtime transactions (zerocopy views)
                        let transactions: Vec<_> = block_bytes
                            .iter()
                            .map(|bytes| to_runtime_transaction(bytes))
                            .collect();
                        let (consumer, recorder, record_receiver) = create_block_consumer(&bank);
                        (consumer, recorder, record_receiver, transactions)
                    },
                    |(mut consumer, _recorder, _record_receiver, transactions)| {
                        let output = consumer.process_and_record_block_transactions(
                            &bank,
                            &transactions,
                            &max_ages,
                            bank.slot(),
                        );
                        // Ensure the block was processed
                        assert!(output
                            .execute_and_commit_transactions_output
                            .commit_transactions_result
                            .is_ok());
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

criterion_group!(
    name = benches;
    config = Criterion::default()
        .noise_threshold(0.1)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(5))
        .sample_size(20);
    targets =
        bench_random_blocks,
        bench_sequential_blocks,
);

criterion_main!(benches);
