use std::env;
use std::error::Error;
use std::process;
use std::ffi::OsString;
use csv::ReaderBuilder;
use serde::Deserialize;
use std::collections::HashMap;


#[derive(Debug, Deserialize)]
struct TransactionRecord {
    #[serde(rename = "type")]
    transaction_type: TransactionType,
    #[serde(rename = "client")]
    client_id: u16,
    #[serde(rename = "tx")]
    transaction_id: u32,
    #[serde(deserialize_with = "csv::invalid_option")] // in case the below tx types have nulls, some absence of value designation etc.
    amount: Option<f64>, //Option because dispute, resolve and chargeback tx types have no associated amount
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum TransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

impl TransactionType {
    fn as_string(&self) -> String {
        match self {
            TransactionType::Deposit => "deposit".to_string(),
            TransactionType::Withdrawal => "withdrawal".to_string(),
            TransactionType::Dispute => "dispute".to_string(),
            TransactionType::Resolve => "resolve".to_string(),
            TransactionType::Chargeback => "chargeback".to_string(),
        }
    }
}

type TransactionKey = u32; //alias for transaction_id for tx hashmap

#[derive(Debug)]
struct TransactionValues {
    client_id: u16,
    amount: Option<f64>,
    in_dispute: bool,
}


type ClientKey = u16; //alias for client_id for accounts hashmap

#[derive(Debug)]
struct ClientAccountValues {
    available: f64,
    held: f64,
    total: f64,
    locked: bool,
}


struct AccountRecord {
    client: u16,
    available: f64,
    held: f64,
    total: f64,
    locked: bool,
}


fn main() {
    if let Err(error) = run_payment_engine() {
        println!("{}", error);
        process::exit(1);
    }
 }


 //TODO: find a more specific error type
 //Pull file path from user input to read transactions to process
fn get_file_path() -> Result<OsString, Box<dyn Error>> {
    match env::args_os().nth(1) {
        None => Err(From::from("expected 1 arguments(the csv file to read from), but got none")),
        Some(file_path) => Ok(file_path),
    }
}


//Engine
fn run_payment_engine() -> Result<(), Box<dyn Error>> {
    let file_path = get_file_path()?; //TODO: Add a check for file path type (ie: confirm it is a .csv)

    parse_csv_into_hashmaps(file_path)?;

    Ok(())
}


//Parse csv into hashmap to run account statement csv generation logic
fn parse_csv_into_hashmaps(file_path: OsString) -> Result<(), Box<dyn Error>> {

    let mut reader = ReaderBuilder::new().trim(csv::Trim::All).from_path(file_path)?; //also trims all whitespace

    let mut account_hashmap: HashMap<ClientKey, ClientAccountValues> = HashMap::new();
    let mut transaction_hashmap: HashMap<TransactionKey, TransactionValues> = HashMap::new();
    
    for record_result in reader.deserialize() {
        let record: TransactionRecord = record_result?;

        match record.transaction_type {
            TransactionType::Deposit => {

                //update an existing entry with the deposit amount if not locked
                if account_hashmap.contains_key(&record.client_id) {
                    let is_locked = account_hashmap.get_key_value(&record.client_id).unwrap().1.locked;//safe to unwrap here, we have already done a contains key check

                    if !is_locked {
                        account_hashmap.entry(record.client_id).and_modify(|a| {
                            a.available += record.amount.unwrap_or(0.0);
                            a.total = a.available + a.held
                        });

                        let transaction_info = TransactionValues {
                            client_id: record.client_id,
                            amount: record.amount,
                            in_dispute: false,
                        };
        
                        //insert deposit transaction info for disputes, resolves and chargebacks to reference later in the loop
                        transaction_hashmap.insert(record.transaction_id, transaction_info);

                    } //We are assuming that if an account is locked then we can disregard deposits to this client account and not write the transaction either
                } else {
                    //create a new entry in the clients hashmap
                    let account_info = ClientAccountValues {
                        available: record.amount.unwrap_or(0.0),
                        held: 0.0000,
                        total: record.amount.unwrap_or(0.0),
                        locked: false,
                    };

                    account_hashmap.insert(record.client_id, account_info);

                    //likewise an entry for the transactions hashmap
                    let transaction_info = TransactionValues {
                        client_id: record.client_id,
                        amount: record.amount,
                        in_dispute: false,
                    };
    
                    //insert deposit transaction info for disputes, resolves and chargebacks to reference later in the loop
                    transaction_hashmap.insert(record.transaction_id, transaction_info);
                }
            },

            TransactionType::Withdrawal => {
                //if sufficient available funds and not locked -> update available and total amounts
                //if client id doesnt not exist in accounts hashmap, being assured by chornology of input csv, we can deduce there is no amount to withdraw -> NoOp
                if account_hashmap.contains_key(&record.client_id) {
                    
                    let is_locked = account_hashmap.get_key_value(&record.client_id).unwrap().1.locked;//safe to unwrap here, we have already done a contains key check
                    let has_sufficient_funds_for_withdrawal = account_hashmap.get_key_value(&record.client_id).unwrap().1.available >= record.amount.unwrap_or(0.0);
                    
                    if !is_locked && has_sufficient_funds_for_withdrawal {
                        account_hashmap.entry(record.client_id).and_modify(|a| {
                            a.available -= record.amount.unwrap_or(0.0);
                            a.total = a.available + a.held
                        });

                        let transaction_info = TransactionValues {
                            client_id: record.client_id,
                            amount: record.amount,
                            in_dispute: false,
                        };
        
                        //insert withdrawal transaction info for disputes, resolves and chargebacks to reference later in the loop
                        transaction_hashmap.insert(record.transaction_id, transaction_info);

                    } //We are assuming that if an account is locked then we can disregard withdrawals from this client account and the same with insufficient funds scenario
                } // else no operation needed and we dont write a transaction that was never processes to tx hashmap
            },
            _ => todo!()
            
        }
        
        
        println!("{:?}", record);
    }
    
    Ok(())
}