use rust_decimal_macros::dec;

use crate::transaction::{Amount, ClientID};
use crate::AccountingError;

#[derive(serde::Serialize, Debug, Clone, PartialEq)]
pub struct Account {
    pub client: ClientID,
    available: Amount,
    held: Amount,
    total: Amount,
    pub locked: bool,
}

impl Default for Account {
    fn default() -> Self {
        Account {
            client: 0,
            available: dec!(0),
            held: dec!(0),
            total: dec!(0),
            locked: false,
        }
    }
}

impl Account {
    #[allow(dead_code)]
    pub fn new(client: ClientID, available: Amount, held: Amount, total: Amount) -> Self {
        Account {
            client,
            available,
            held,
            total,
            locked: false,
        }
    }

    pub fn deposit(&mut self, amount: Amount) {
        self.available += amount;
        self.total += amount;
    }

    pub fn withdrawal(&mut self, amount: Amount) -> Result<(), AccountingError> {
        if self.available - amount >= dec!(0) {
            self.available -= amount;
            self.total -= amount;
            return Ok(());
        }
        Err(AccountingError::Withdrawal)
    }

    // Logic around existing tx etc. should be handled elsewhere
    pub fn dispute(&mut self, amount: Amount) -> Result<(), AccountingError> {
        if self.available >= amount {
            self.held += amount;
            self.available -= amount;
        } else {
            return Err(AccountingError::Dispute);
        }
        Ok(())
    }

    pub fn resolve(&mut self, amount: Amount) {
        self.held -= amount;
        self.available += amount;
    }

    pub fn chargeback(&mut self, amount: Amount) {
        self.held -= amount;
        self.total -= amount;
        self.locked = true;
    }

    pub fn normalize_values(&mut self) {
        self.available = self.available.normalize();
        self.held = self.held.normalize();
        self.total = self.total.normalize();
    }
}
