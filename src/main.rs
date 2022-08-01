use std::collections::BTreeMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use account::Account;
use csv_utils::TransactionReader;
pub use error::AccountingError;
use transaction::{ClientID, TransactionData, TxID};
use transaction_processor::TransactionProcessor;

mod account;
mod csv_utils;
mod error;
mod transaction;
mod transaction_processor;

#[tokio::main]
async fn main() -> Result<(), AccountingError> {
    // Let the ledger live throughout the lifetime of the program, and be shared between
    // all transaction processors (in the event of multiple incoming connections etc.)
    let ledger: Arc<RwLock<BTreeMap<TxID, TransactionData>>> = Default::default();
    let accounts: Arc<RwLock<BTreeMap<ClientID, Account>>> = Default::default();

    let input_path = std::env::args()
        .nth(1)
        .expect("error: missing input file path");
    if let Ok(mut reader) = TransactionReader::new(input_path) {
        // Create the transaction processor for this input stream
        let (processor, sender) = TransactionProcessor::new(ledger.clone(), accounts.clone());
        // Spawn a new thread for the processor, and let it await incoming data
        let processor: JoinHandle<TransactionProcessor> =
            tokio::spawn(async move { processor.process().await });

        let processor_handle = processor;

        loop {
            match reader.get_next_record() {
                Ok(Some(tx)) => {
                    sender
                        .send(tx)
                        .map_err(|err| AccountingError::TokioChannel(err.to_string()))?;
                }
                Err(_e) => {
                    // Log error, commented out for now to avoid clobbering stdout
                    //eprintln!("Error: {}", _e);
                }
                // Done, no more records
                Ok(None) => break,
            }
        }

        drop(sender);
        match processor_handle.await {
            Ok(_processor) => (),
            Err(e) => return Err(AccountingError::HandleAwait(e.to_string())),
        }

        let accounts_output = accounts.read().await;
        let output = accounts_output
            .clone()
            .into_iter()
            .map(|(_, v)| v)
            .collect::<Vec<Account>>();
        csv_utils::print_output(output);
    }
    Ok(())
}
