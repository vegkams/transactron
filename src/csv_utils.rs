use std::error::Error;
use std::fs::File;
use std::io::BufReader;

use rust_decimal_macros::dec;

use crate::transaction::{Amount, ClientID, Transaction, TransactionData, TxID};
use crate::Account;
use crate::AccountingError;

pub struct TransactionReader {
    bufreader: csv::Reader<BufReader<File>>,
}

impl TransactionReader {
    // Creates and returns a buffered csv reader, avoids loading the entire input file into memory
    pub fn new(csv_path: String) -> Result<Self, Box<dyn Error>> {
        let file = File::open(csv_path)?;
        let buffered_reader = BufReader::new(file);
        let csv_reader = csv::ReaderBuilder::new()
            .trim(csv::Trim::All)
            .delimiter(b',')
            .has_headers(true)
            .flexible(true)
            .from_reader(buffered_reader);
        Ok(TransactionReader {
            bufreader: csv_reader,
        })
    }

    pub fn get_next_record(&mut self) -> Result<Option<Transaction>, AccountingError> {
        if let Some(record) = self.bufreader.deserialize().next() {
            let event: Record = match record {
                Ok(r) => r,
                Err(e) => return Err(AccountingError::Deserialize(e.to_string())),
            };

            if let Some(tx) = TransactionReader::record_to_transaction(event) {
                return Ok(Some(tx));
            } else {
                return Err(AccountingError::MalformedTransaction);
            }
        }
        // No more transactions should not be an error, so return Ok(None)
        Ok(None)
    }

    // Transforms the Record struct into the Transaction enum with inner TransactionData
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
}
pub fn print_output(output: Vec<Account>) {
    let mut writer = csv::Writer::from_writer(std::io::stdout());
    for mut account in output {
        account.normalize_values();
        writer.serialize(account).unwrap();
    }
    writer.flush().unwrap();
}

#[derive(serde::Deserialize, Debug)]
struct Record {
    #[serde(rename = "type")]
    transaction_type: Option<String>,
    client: Option<ClientID>,
    tx: Option<TxID>,
    amount: Option<Amount>,
}
