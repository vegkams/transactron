use rust_decimal::prelude::*;

#[derive(Clone)]
pub enum Transaction {
    Deposit(TransactionData),
    Withdrawal(TransactionData),
    Resolve(TransactionData),
    Dispute(TransactionData),
    Chargeback(TransactionData),
}

#[derive(Clone)]
pub struct TransactionData {
    pub client_id: u16,
    pub tx_id: u32,
    pub amount: Option<Decimal>,
    pub under_dispute: bool,
}
