# transactron

A MVP accounting system that currently reads transactions as .csv files provided as a path on stdin,
and handles deposits, withdrawals, disputes, resolves and chargebacks for an arbitrary amount of accounts.

The system is designed to handle multiple inputs with minimal change, due to globally shared state and a dedicated processing engine thread per input.

# Build
```commandline
cargo build
```

# Test
```commandline
cargo test
```
Tests the core transaction processor.

# Use
```commandline
cargo run /path/to/transactions.csv
```
The csv file is in the following format:
- `type` The transaction type (string): 
One of deposit, withdrawal, dispute, resolve, and chargeback. 
Only *deposit* and *withdrawal* specify their own transaction id and amount. 
Every other type references a previous transaction id and no amount
- `client` Client Id (u16): A globally unique identifier for the client account
- `tx` Transaction Id (u32): A globally unique identifier for the transaction
- `amount` Transaction Amount (decimal with precision up to four places after the decimal)


The output, representing the accounts state as a .csv, have the following columns:
- `client` Client Id (u16)
- `available` Available Funds (decimal)
- `held` Funds in Dispute (decimal)
- `total`=`available`+`held` (decimal)
- `locked` If a chargeback happens, the account is frozen, represented by this column (bool)

Assumptions:

* A chargeback may result in negative balance
* Transactions in csv may be malformed. Malformed transactions are quietly ignored.
* Amounts in transactions should be strictly positive values. Negative or zero values in deposits or withdrawals are quietly ignored.
