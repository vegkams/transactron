use rust_decimal::prelude::*;
use std::ops::Deref;

#[derive(Clone, Debug)]
pub enum Transaction {
    Deposit(TransactionData),
    Withdrawal(TransactionData),
    Resolve(TransactionData),
    Dispute(TransactionData),
    Chargeback(TransactionData),
}

impl Deref for Transaction {
    type Target = TransactionData;
    fn deref(&self) -> &TransactionData {
        match self {
            Transaction::Deposit(tx) => tx,
            Transaction::Withdrawal(tx) => tx,
            Transaction::Resolve(tx) => tx,
            Transaction::Dispute(tx) => tx,
            Transaction::Chargeback(tx) => tx,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TransactionData {
    pub client_id: u16,
    pub tx_id: u32,
    pub amount: Option<Decimal>,
    pub under_dispute: bool,
}
