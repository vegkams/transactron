use rust_decimal_macros::dec;
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
        while let Some(tx) = self.transaction_recv.recv().await {
            match self.process_transaction(tx).await {
                // TODO: Error handling
                Ok(_) => (),
                Err(_) => (),
            };
        }

        self
    }

    async fn try_write_to_ledger(&mut self, tx: TransactionData) -> Result<(), AccountingError> {
        let mut transactions = self.transactions.write().await;
        if let Entry::Vacant(e) = transactions.entry(tx.tx_id) {
            e.insert(tx);
        } else {
            return Err(AccountingError::TransactionAlreadyExists);
        }
        Ok(())
    }

    pub async fn process_transaction(&mut self, tx: Transaction) -> Result<(), AccountingError> {
        let client_id = match tx.clone() {
            Transaction::Deposit(tx_data) => {
                self.try_write_to_ledger(tx_data.clone()).await?;
                tx_data.client_id
            }
            Transaction::Withdrawal(tx_data) => {
                self.try_write_to_ledger(tx_data.clone()).await?;
                tx_data.client_id
            }
            Transaction::Dispute(tx_data) => tx_data.client_id,
            Transaction::Resolve(tx_data) => tx_data.client_id,
            Transaction::Chargeback(tx_data) => tx_data.client_id,
        };

        let mut accounts = self.accounts.write().await;
        let mut client = accounts.entry(client_id).or_default();
        if client.client != client_id {
            client.client = client_id;
        }

        if client.locked {
            return Err(AccountingError::AccountLocked);
        }

        match tx {
            Transaction::Deposit(tx_data) => {
                // Safe to unwrap because of the check performed when the Transaction was created
                let amount = tx_data.amount.unwrap();
                if amount > dec!(0) {
                    client.deposit(amount)
                } else {
                    return Err(AccountingError::DepositError);
                }
            }
            Transaction::Withdrawal(tx_data) => {
                let amount = tx_data.amount.unwrap();
                if amount > dec!(0) {
                    client.withdrawal(tx_data.amount.unwrap())?;
                } else {
                    return Err(AccountingError::WithdrawalError);
                }
            }
            Transaction::Dispute(tx_data) => {
                let mut transactions = self.transactions.write().await;
                if let Some(mut t) = transactions.get_mut(&tx_data.tx_id) {
                    // Transaction under dispute exists in the ledger
                    if t.amount.is_some() && !t.under_dispute {
                        // Dispute the amount iff this is a transaction with an associated amount (i.e. Deposit or Withdrawal)
                        client.dispute(t.amount.unwrap());
                        t.under_dispute = true;
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
            &Account::new(
                1u16,
                dec!(1.5),
                dec!(0),
                dec!(1.5),
            ),
            output.get(0).unwrap()
        );
    }
}
