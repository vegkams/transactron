use account::Account;
use rust_decimal::prelude::*;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use transaction::{Transaction, TransactionData};
use transaction_processor::TransactionProcessor;

pub use error::AccountingError;

mod account;
mod error;
mod transaction;
mod transaction_processor;

#[tokio::main]
async fn main() -> Result<(), AccountingError> {
    // Let the ledger live throughout the lifetime of the program, and be shared between
    // all transaction processors (in the event of multiple incoming connections etc.)
    let ledger: Arc<RwLock<BTreeMap<u32, TransactionData>>> = Default::default();
    let accounts: Arc<RwLock<BTreeMap<u16, Account>>> = Default::default();
    let mut processor_handles: Vec<JoinHandle<TransactionProcessor>> = vec![];

    if let Some(input_path) = std::env::args().nth(1) {
        if let Ok(mut reader) = buffered_tx_reader(input_path) {
            // Create the transaction processor for this input stream
            let (processor, sender) = TransactionProcessor::new(ledger.clone(), accounts.clone());
            let processor: JoinHandle<TransactionProcessor> =
                tokio::spawn(async move { processor.process().await });

            processor_handles.push(processor);

            for record in reader.deserialize() {
                let event: Record = match record {
                    Ok(r) => r,
                    Err(_) => return Err(AccountingError::DeserializeError),
                };

                if let Some(tx) = record_to_transaction(event) {
                    sender
                        .send(tx)
                        .map_err(|err| AccountingError::TokioChannelError(err.to_string()))?;
                };
            }
            drop(sender);
            for handle in processor_handles {
                if let Ok(processor) = handle.await {
                    let output = processor.get_accounts_state().await;
                    print_output(output);
                }
            }
        }
    }
    Ok(())
}

fn print_output(output: Vec<Account>) {
    let mut writer = csv::Writer::from_writer(std::io::stdout());
    for account in output {
        writer.serialize(account).unwrap();
    }
    writer.flush().unwrap();
}

fn buffered_tx_reader(csv_path: String) -> Result<csv::Reader<BufReader<File>>, Box<dyn Error>> {
    let file = File::open(csv_path)?;
    let buffered_reader = BufReader::new(file);
    let csv_reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(buffered_reader);
    Ok(csv_reader)
}

fn record_to_transaction(record: Record) -> Option<Transaction> {
    match record.transaction_type.as_str() {
        "deposit" => {
            record.amount?;
            Some(Transaction::Deposit(TransactionData {
                client_id: record.client,
                tx_id: record.tx,
                amount: Some(record.amount.unwrap()),
                under_dispute: false,
            }))
        }
        "withdrawal" => {
            record.amount?;
            Some(Transaction::Withdrawal(TransactionData {
                client_id: record.client,
                tx_id: record.tx,
                amount: Some(record.amount.unwrap()),
                under_dispute: false,
            }))
        }
        "dispute" => Some(Transaction::Dispute(TransactionData {
            client_id: record.client,
            tx_id: record.tx,
            amount: None,
            under_dispute: false,
        })),
        "resolve" => Some(Transaction::Resolve(TransactionData {
            client_id: record.client,
            tx_id: record.tx,
            amount: None,
            under_dispute: false,
        })),
        "chargeback" => Some(Transaction::Chargeback(TransactionData {
            client_id: record.client,
            tx_id: record.tx,
            amount: None,
            under_dispute: false,
        })),
        _ => None,
    }
}

#[derive(serde::Deserialize, Debug)]
pub struct Record {
    #[serde(rename = "type")]
    transaction_type: String,
    client: u16,
    tx: u32,
    amount: Option<Decimal>,
}
