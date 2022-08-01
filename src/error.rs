use thiserror::Error;

#[derive(Debug, Error, Clone)]
pub enum AccountingError {
    #[error("Error: Insufficient funds for withdrawal")]
    Withdrawal,
    #[error("Error: Deposit transaction without an amount")]
    Deposit,
    #[error("Error: Insufficient funds for dispute")]
    Dispute,
    #[error("Error: Could not deserialize record: {0}")]
    Deserialize(String),
    #[error("Error: malformed transaction")]
    MalformedTransaction,
    #[error("Error: Could not send tx data to worker: {0}")]
    TokioChannel(String),
    #[error("Error: The transaction already exists in the ledger")]
    TransactionAlreadyExists,
    #[error("Error: Account is locked")]
    AccountLocked,
    #[error("Error: Processor future returned error: {0}")]
    HandleAwait(String),
}
