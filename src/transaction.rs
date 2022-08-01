use rust_decimal::prelude::*;
use std::ops::Deref;

pub type TxID = u32;
pub type ClientID = u16;
pub type Amount = Decimal;

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
    pub client_id: ClientID,
    pub tx_id: TxID,
    pub amount: Option<Amount>,
    pub under_dispute: bool,
}
