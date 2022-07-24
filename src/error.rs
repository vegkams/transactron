use thiserror::Error;

#[derive(Debug, Error, Clone)]
pub enum AccountingError {
    #[error("Error: Insufficient funds for withdrawal")]
    WithdrawalError,
    #[error("Error: Deposit transaction without an amount")]
    DepositError,
    #[error("Error: Could not deserialize record: {0}")]
    DeserializeError(String),
    #[error("Error: Could not send tx data to worker: {0}")]
    TokioChannelError(String),
    #[error("Error: The transaction already exists in the ledger")]
    TransactionAlreadyExists,
    #[error("Error: Account is locked")]
    AccountLocked,
    #[error("Error: Processor future returned error: {0}")]
    HandleAwaitError(String),
}
