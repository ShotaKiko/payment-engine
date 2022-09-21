# Payment-Engine
A simple csv transaction engine that outputs client account statement information.

##Basics
Buildable via `cargo build`

Runnable with output to stdout via:
    
    cargo run <your_csv.csv>

* ex. `cargo run transactions.csv`
* a csv needs to be provided or process will exit

Runnable with output to a specified csv via:

    cargo run -- <input_csv.csv> > <csv_to_write_to.csv>

* ex. `cargo run -- transactions.csv > accounts.csv`
* a csv needs to be provided or process will exit
* a csv will be generated in the root directory following running this command


### CSV Details
The input CSV file must consist of 4 columns.

* `type` -- transaction type. Valid types being `deposit`, `withdrawal`, `dispute`, `resolve`, `chargeback`
* `client` -- the client id `integer` associated with this transaction
* `tx` -- the transaction id `integer` of the transaction itself 
* `amount` -- the amount `decimal` associated with the transaction


### What the engine can handle
The engine should handle the following transactions with said business logic assuming the client account is not locked from a chargeback transaction.
* `deposits` -- adds tx amount to client account available pool
* `withdrawals` -- will remove tx amount from client account available pool if available funds are sufficent
* `disputes` -- will moved the specified tx's amount from a clients available pool to held pool 
* `resolutions` -- reverses a disputed tx, moving held funds back into available (if dispute not found this tx is ignored)
* `chargebacks` -- funds held will be withdrawn and removed from total pool. Also account will be locked (^^^ditto^^^)

###Testing the engine
Developed testing manually with csvs included in the repo. Currently no unit tests in place. Engine relies on type system to ensure correctness. A great future feature would be unit tests with premade transaction and account csvs and assertions that all transactions are handled properly. As of now all testing was done manually without any controls.

There is a folder with example cvs. Please feel fre to use them to test the most basic situations manually for yourself via:

    cargo run csvs/<csv_name>.csv

###Drawbacks
In terms of efficiency the engine currently loads the entire data set into memory. Streaming values instead would be an improvement.

The current flow is to load a csv, loop through the records, generating hashmaps for accounts and transactions. Following completion of the first loop it loops through the generated accounts hashmap and writes to a csv or stdout.

Taking this into consideration a general complexity analysis is:
* time -- `O(n + ah)` where `n` is the `number of records in the supplied csv` and `ah` is the `number of entries in the generated account hashmap`
* space -- `O(n + ah + th)` where `n` is the `size of thecsv` and `ah` is the `size of the account hashmap` and `th` is the `size of the transactions hashtable` used for processing `disputes`, `resolutions` and `chargebacks`.

###Future nice to haves
* abstract some of the repeated business logic checks found in all the match arms into general helper functions
* have a more concrete/useful error type than a `Box<dyn Error>`
* stream the csv data values as opposed to loading the whole data set
* write some unit tests from control csvs with assertions for correctness
* add further check on input arg confirming supplied file has .csv extention
