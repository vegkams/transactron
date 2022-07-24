use std::collections::{btree_map::Entry, BTreeMap};
use std::sync::Arc;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::sync::RwLock;

use crate::account::Account;
use crate::transaction::{Transaction, TransactionData};
use crate::AccountingError;

pub struct TransactionProcessor {
    accounts: Arc<RwLock<BTreeMap<u16, Account>>>,
    transactions: Arc<RwLock<BTreeMap<u32, TransactionData>>>,
    transaction_recv: UnboundedReceiver<Transaction>,
}

impl TransactionProcessor {
    pub fn new(
        transactions: Arc<RwLock<BTreeMap<u32, TransactionData>>>,
        accounts: Arc<RwLock<BTreeMap<u16, Account>>>,
    ) -> (Self, UnboundedSender<Transaction>) {
        let (sender, receiver) = unbounded_channel();
        (
            TransactionProcessor {
                accounts,
                transactions,
                transaction_recv: receiver,
            },
            sender,
        )
    }

    pub async fn process(mut self) -> Self {
        // loop until sender is dropped
        while let Some(tx) = self.transaction_recv.recv().await {
            match self.process_transaction(tx).await {
                // TODO: Error handling
                Ok(_) => (),
                Err(_e) => {
                    // Todo: Do more sophisticated error handling. Write the erroneous transaction to log etc.
                    // eprintln!("Error processing transaction: {:?}", _e);
                }
            };
        }

        self
    }

    pub async fn process_transaction(&mut self, tx: Transaction) -> Result<(), AccountingError> {
        let client_id = tx.client_id;

        let mut accounts = self.accounts.write().await;
        // Create new client with default values if it doesn't already exist
        let mut client = accounts.entry(client_id).or_default();
        if client.client != client_id {
            // New client, set correct client id
            client.client = client_id;
        }

        if client.locked {
            return Err(AccountingError::AccountLocked);
        }

        match tx {
            Transaction::Deposit(tx_data) => {
                // Safe to unwrap because of the check performed when the Transaction was created
                client.deposit(tx_data.amount.unwrap());
                let mut transactions = self.transactions.write().await;
                if let Entry::Vacant(e) = transactions.entry(tx_data.tx_id) {
                    e.insert(tx_data);
                } else {
                    return Err(AccountingError::TransactionAlreadyExists);
                }
            }
            Transaction::Withdrawal(tx_data) => {
                // This can fail if the amount exceeds the available amount in the account
                client.withdrawal(tx_data.amount.unwrap())?;
                let mut transactions = self.transactions.write().await;
                if let Entry::Vacant(e) = transactions.entry(tx_data.tx_id) {
                    e.insert(tx_data);
                } else {
                    return Err(AccountingError::TransactionAlreadyExists);
                }
            }
            Transaction::Dispute(tx_data) => {
                let mut transactions = self.transactions.write().await;
                if let Some(mut t) = transactions.get_mut(&tx_data.tx_id) {
                    // Transaction under dispute exists in the ledger
                    if t.amount.is_some() && !t.under_dispute {
                        // Dispute the amount iff this is a transaction with an associated amount (i.e. Deposit or Withdrawal)
                        // and there are sufficient funds available to be held
                        match client.dispute(t.amount.unwrap()) {
                            Ok(()) => t.under_dispute = true,
                            Err(e) => return Err(e),
                        }
                    } // else ignore since it is an error on partners side
                }
            }
            Transaction::Resolve(tx_data) => {
                let mut transactions = self.transactions.write().await;
                if let Some(mut t) = transactions.get_mut(&tx_data.tx_id) {
                    // Transaction under dispute exists in the ledger
                    if t.amount.is_some() && t.under_dispute {
                        // Dispute the amount iff this is a transaction with an associated amount (i.e. Deposit or Withdrawal)
                        client.resolve(t.amount.unwrap());
                        t.under_dispute = false;
                    } // else ignore since it is an error on partners side
                }
            }
            Transaction::Chargeback(tx_data) => {
                let mut transactions = self.transactions.write().await;
                if let Some(mut t) = transactions.get_mut(&tx_data.tx_id) {
                    // Transaction under dispute exists in the ledger
                    if t.amount.is_some() && t.under_dispute {
                        // Dispute the amount iff this is a transaction with an associated amount (i.e. Deposit or Withdrawal)
                        client.chargeback(t.amount.unwrap());
                        t.under_dispute = false;
                        client.locked = true;
                    } // else ignore since it is an error on partners side
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use tokio::task::JoinHandle;
    use rust_decimal_macros::dec;

    #[tokio::test]
    async fn test_one_deposit() {
        let ledger: Arc<RwLock<BTreeMap<u32, TransactionData>>> = Default::default();
        let accounts: Arc<RwLock<BTreeMap<u16, Account>>> = Default::default();
        let (processor, sender) = TransactionProcessor::new(ledger.clone(), accounts.clone());
        let processor: JoinHandle<TransactionProcessor> =
            tokio::spawn(async move { processor.process().await });
        sender
            .send(Transaction::Deposit(TransactionData {
                client_id: 1,
                tx_id: 1,
                amount: Some(dec!(1.5)),
                under_dispute: false,
            }))
            .unwrap();
        drop(sender);
        processor.await.unwrap();

        let accounts_output = accounts.read().await;
        let output = accounts_output
            .clone()
            .into_iter()
            .map(|(_, v)| v)
            .collect::<Vec<Account>>();
        assert_eq!(
            &Account::new(1u16, dec!(1.5), dec!(0), dec!(1.5),),
            output.get(0).unwrap()
        );
    }

    #[tokio::test]
    async fn test_two_deposits_and_one_withdrawal() {
        let ledger: Arc<RwLock<BTreeMap<u32, TransactionData>>> = Default::default();
        let accounts: Arc<RwLock<BTreeMap<u16, Account>>> = Default::default();
        let (processor, sender) = TransactionProcessor::new(ledger.clone(), accounts.clone());
        let processor: JoinHandle<TransactionProcessor> =
            tokio::spawn(async move { processor.process().await });
        sender
            .send(Transaction::Deposit(TransactionData {
                client_id: 1,
                tx_id: 1,
                amount: Some(dec!(1.5)),
                under_dispute: false,
            }))
            .unwrap();
        sender
            .send(Transaction::Deposit(TransactionData {
                client_id: 2,
                tx_id: 2,
                amount: Some(dec!(3.3333)),
                under_dispute: false,
            }))
            .unwrap();
        sender
            .send(Transaction::Withdrawal(TransactionData {
                client_id: 2,
                tx_id: 3,
                amount: Some(dec!(1)),
                under_dispute: false,
            }))
            .unwrap();
        drop(sender);
        processor.await.unwrap();

        let accounts_output = accounts.read().await;
        let output = accounts_output
            .clone()
            .into_iter()
            .map(|(_, v)| v)
            .collect::<Vec<Account>>();

        assert_eq!(
            &Account::new(1u16, dec!(1.5), dec!(0), dec!(1.5),),
            output.get(0).unwrap()
        );
        assert_eq!(
            &Account::new(2u16, dec!(2.3333), dec!(0), dec!(2.3333),),
            output.get(1).unwrap()
        );
    }

    #[tokio::test]
    async fn test_dispute() {
        let ledger: Arc<RwLock<BTreeMap<u32, TransactionData>>> = Default::default();
        let accounts: Arc<RwLock<BTreeMap<u16, Account>>> = Default::default();
        let (processor, sender) = TransactionProcessor::new(ledger.clone(), accounts.clone());
        let processor: JoinHandle<TransactionProcessor> =
            tokio::spawn(async move { processor.process().await });
        sender
            .send(Transaction::Deposit(TransactionData {
                client_id: 1,
                tx_id: 1,
                amount: Some(dec!(1.5)),
                under_dispute: false,
            }))
            .unwrap();
        sender
            .send(Transaction::Deposit(TransactionData {
                client_id: 1,
                tx_id: 2,
                amount: Some(dec!(3)),
                under_dispute: false,
            }))
            .unwrap();
        sender
            .send(Transaction::Dispute(TransactionData {
                client_id: 1,
                tx_id: 2,
                amount: None,
                under_dispute: false,
            }))
            .unwrap();

        drop(sender);
        processor.await.unwrap();

        let accounts_output = accounts.read().await;
        let output = accounts_output
            .clone()
            .into_iter()
            .map(|(_, v)| v)
            .collect::<Vec<Account>>();
        assert_eq!(
            &Account::new(1u16, dec!(1.5), dec!(3), dec!(4.5),),
            output.get(0).unwrap()
        );
    }

    #[tokio::test]
    async fn test_chargeback() {
        let ledger: Arc<RwLock<BTreeMap<u32, TransactionData>>> = Default::default();
        let accounts: Arc<RwLock<BTreeMap<u16, Account>>> = Default::default();
        let (processor, sender) = TransactionProcessor::new(ledger.clone(), accounts.clone());
        let processor: JoinHandle<TransactionProcessor> =
            tokio::spawn(async move { processor.process().await });
        sender
            .send(Transaction::Deposit(TransactionData {
                client_id: 1,
                tx_id: 1,
                amount: Some(dec!(1.5)),
                under_dispute: false,
            }))
            .unwrap();
        sender
            .send(Transaction::Deposit(TransactionData {
                client_id: 1,
                tx_id: 2,
                amount: Some(dec!(3)),
                under_dispute: false,
            }))
            .unwrap();
        sender
            .send(Transaction::Dispute(TransactionData {
                client_id: 1,
                tx_id: 2,
                amount: None,
                under_dispute: false,
            }))
            .unwrap();
        sender
            .send(Transaction::Chargeback(TransactionData {
                client_id: 1,
                tx_id: 2,
                amount: None,
                under_dispute: false,
            }))
            .unwrap();

        drop(sender);
        processor.await.unwrap();

        let accounts_output = accounts.read().await;
        let output = accounts_output
            .clone()
            .into_iter()
            .map(|(_, v)| v)
            .collect::<Vec<Account>>();

        let mut account = Account::new(1u16, dec!(1.5), dec!(0), dec!(1.5));
        account.locked = true;
        assert_eq!(&account, output.get(0).unwrap());
    }

    #[tokio::test]
    async fn test_resolve() {
        let ledger: Arc<RwLock<BTreeMap<u32, TransactionData>>> = Default::default();
        let accounts: Arc<RwLock<BTreeMap<u16, Account>>> = Default::default();
        let (processor, sender) = TransactionProcessor::new(ledger.clone(), accounts.clone());
        let processor: JoinHandle<TransactionProcessor> =
            tokio::spawn(async move { processor.process().await });
        sender
            .send(Transaction::Deposit(TransactionData {
                client_id: 1,
                tx_id: 1,
                amount: Some(dec!(1.5)),
                under_dispute: false,
            }))
            .unwrap();
        sender
            .send(Transaction::Deposit(TransactionData {
                client_id: 1,
                tx_id: 2,
                amount: Some(dec!(3)),
                under_dispute: false,
            }))
            .unwrap();
        sender
            .send(Transaction::Dispute(TransactionData {
                client_id: 1,
                tx_id: 2,
                amount: None,
                under_dispute: false,
            }))
            .unwrap();
        sender
            .send(Transaction::Resolve(TransactionData {
                client_id: 1,
                tx_id: 2,
                amount: None,
                under_dispute: false,
            }))
            .unwrap();

        drop(sender);
        processor.await.unwrap();

        let accounts_output = accounts.read().await;
        let output = accounts_output
            .clone()
            .into_iter()
            .map(|(_, v)| v)
            .collect::<Vec<Account>>();

        assert_eq!(
            &Account::new(1u16, dec!(4.5), dec!(0), dec!(4.5)),
            output.get(0).unwrap()
        );
    }
}
