use rust_decimal::prelude::*;
use rust_decimal_macros::dec;
use std::collections::BTreeMap;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use account::Account;
pub use error::AccountingError;
use transaction::{Transaction, TransactionData};
use transaction_processor::TransactionProcessor;

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
            // Spawn a new thread for the processor, and let it await incoming data
            let processor: JoinHandle<TransactionProcessor> =
                tokio::spawn(async move { processor.process().await });

            processor_handles.push(processor);

            for record in reader.deserialize() {
                let event: Record = match record {
                    Ok(r) => r,
                    Err(e) => return Err(AccountingError::DeserializeError(e.to_string())),
                };

                if let Some(tx) = record_to_transaction(event) {
                    sender
                        .send(tx)
                        .map_err(|err| AccountingError::TokioChannelError(err.to_string()))?;
                };
            }
            drop(sender);
            for handle in processor_handles {
                match handle.await {
                    Ok(_processor) => (),
                    Err(e) => return Err(AccountingError::HandleAwaitError(e.to_string())),
                }
            }
            let accounts_output = accounts.read().await;
            let output = accounts_output
                .clone()
                .into_iter()
                .map(|(_, v)| v)
                .collect::<Vec<Account>>();
            print_output(output);
        }
    }
    Ok(())
}

fn print_output(output: Vec<Account>) {
    let mut writer = csv::Writer::from_writer(std::io::stdout());
    for mut account in output {
        account.normalize_values();
        writer.serialize(account).unwrap();
    }
    writer.flush().unwrap();
}

fn buffered_tx_reader(csv_path: String) -> Result<csv::Reader<BufReader<File>>, Box<dyn Error>> {
    let file = File::open(csv_path)?;
    let buffered_reader = BufReader::new(file);
    let csv_reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .delimiter(b',')
        .has_headers(true)
        .flexible(true)
        .from_reader(buffered_reader);
    Ok(csv_reader)
}

fn record_to_transaction(record: Record) -> Option<Transaction> {
    if let Some(transaction_type) = record.transaction_type {
        match transaction_type.as_str() {
            "deposit" => {
                record.amount?;
                record.client?;
                record.tx?;
                if record.amount.unwrap() <= dec!(0) {
                    return None;
                }
                Some(Transaction::Deposit(TransactionData {
                    client_id: record.client.unwrap(),
                    tx_id: record.tx.unwrap(),
                    amount: Some(record.amount.unwrap()),
                    under_dispute: false,
                }))
            }
            "withdrawal" => {
                record.amount?;
                record.client?;
                record.tx?;
                if record.amount.unwrap() <= dec!(0) {
                    return None;
                }
                Some(Transaction::Withdrawal(TransactionData {
                    client_id: record.client.unwrap(),
                    tx_id: record.tx.unwrap(),
                    amount: Some(record.amount.unwrap()),
                    under_dispute: false,
                }))
            }
            "dispute" => {
                record.client?;
                record.tx?;
                Some(Transaction::Dispute(TransactionData {
                    client_id: record.client.unwrap(),
                    tx_id: record.tx.unwrap(),
                    amount: None,
                    under_dispute: false,
                }))
            }
            "resolve" => {
                record.client?;
                record.tx?;
                Some(Transaction::Resolve(TransactionData {
                    client_id: record.client.unwrap(),
                    tx_id: record.tx.unwrap(),
                    amount: None,
                    under_dispute: false,
                }))
            }
            "chargeback" => {
                record.client?;
                record.tx?;
                Some(Transaction::Chargeback(TransactionData {
                    client_id: record.client.unwrap(),
                    tx_id: record.tx.unwrap(),
                    amount: None,
                    under_dispute: false,
                }))
            }
            _ => None,
        }
    } else {
        None
    }
}

#[derive(serde::Deserialize, Debug)]
pub struct Record {
    #[serde(rename = "type")]
    transaction_type: Option<String>,
    client: Option<u16>,
    tx: Option<u32>,
    amount: Option<Decimal>,
}
