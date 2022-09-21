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


#[derive(Debug)]
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
            //Note to reader: We're opting for option unwraps over if lets because I believe the former is more readable in this specific case because of all the business logic checks we need. 
            //See Dispute match arm for commented version of if let logic
            
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

            TransactionType::Dispute => {
                //query tx hashmap by tx id as key
                //if found update account with associated client id and tx dispute bool
                if transaction_hashmap.contains_key(&record.transaction_id) {
                    let associated_client_id = transaction_hashmap.get_key_value(&record.transaction_id).unwrap().1.client_id;// safe to unwrap here because we have checked for key
                    let associated_amount = transaction_hashmap.get_key_value(&record.transaction_id).unwrap().1.amount;

                    if associated_amount.is_some() {
                        let amount = associated_amount.unwrap(); //safe to unwrap after is_some check
                        
                        if account_hashmap.contains_key(&associated_client_id) {
                    
                            let is_locked = account_hashmap.get_key_value(&associated_client_id).unwrap().1.locked;//safe to unwrap here, we have already done a contains key check    
                            if !is_locked {
                                account_hashmap.entry(associated_client_id).and_modify(|a| {
                                    a.available -= amount;
                                    a.held += amount;
                                    a.total = a.available + a.held
                                });

                                transaction_hashmap.entry(record.transaction_id).and_modify(|t| t.in_dispute = true); //update this transaction as disputed
                            }
                        }
                    }
                }
                
                //Below is the same logic implemented with if lets. if let is usually favored over the safe option unwrap but regardless I think the above is easier to read/follow
                // if transaction_hashmap.contains_key(&record.transaction_id) {

                //     if let Some(transaction_kv) = transaction_hashmap.get_key_value(&record.transaction_id){
                //         let associated_client_id = transaction_kv.1.client_id;

                //         if let Some(associated_amount) = transaction_kv.1.amount {

                //             if account_hashmap.contains_key(&associated_client_id){

                //                 if let Some(account_kv) = account_hashmap.get_key_value(&associated_client_id) {
                //                     let is_locked = account_kv.1.locked;

                //                     if !is_locked {
                //                         account_hashmap.entry(associated_client_id).and_modify(|a| {
                //                             a.available -= associated_amount;
                //                             a.held += associated_amount;
                //                             a.total = a.available + a.held
                //                         });
                    
                //                          transaction_hashmap.entry(record.transaction_id).and_modify(|t| t.in_dispute = true);

                //                     } 
                //                 }
                //             }
                //         }
                //     }
                // }
            },

            TransactionType::Resolve => {
                //query tx hashmap by tx id as key
                //if found update account with associated client id, releasing held amount and set tx dispute bool to false again
                if transaction_hashmap.contains_key(&record.transaction_id) {
                    let associated_client_id = transaction_hashmap.get_key_value(&record.transaction_id).unwrap().1.client_id;// safe to unwrap here because we have checked for key
                    let associated_amount = transaction_hashmap.get_key_value(&record.transaction_id).unwrap().1.amount;
                    let tx_in_dispute = transaction_hashmap.get_key_value(&record.transaction_id).unwrap().1.in_dispute;

                    if associated_amount.is_some() && tx_in_dispute { //if this tx is not in dispute we can skip all proceeding logic
                        let amount = associated_amount.unwrap(); //safe to unwrap after is_some check
                        
                        if account_hashmap.contains_key(&associated_client_id) {
                    
                            let is_locked = account_hashmap.get_key_value(&associated_client_id).unwrap().1.locked;//safe to unwrap here, we have already done a contains key check    
                            if !is_locked {
                                account_hashmap.entry(associated_client_id).and_modify(|a| {
                                    a.available += amount;
                                    a.held -= amount;
                                });

                                transaction_hashmap.entry(record.transaction_id).and_modify(|t| t.in_dispute = false); //update this transaction as disputed
                            }
                        }
                    }
                }
            },

            TransactionType::Chargeback => {
                //query tx hashmap by tx.id
                //if exists and in dispute update held and total amount, locking associated client account
                if transaction_hashmap.contains_key(&record.transaction_id) {
                    let associated_client_id = transaction_hashmap.get_key_value(&record.transaction_id).unwrap().1.client_id;// safe to unwrap here because we have checked for key
                    let associated_amount = transaction_hashmap.get_key_value(&record.transaction_id).unwrap().1.amount;
                    let tx_in_dispute = transaction_hashmap.get_key_value(&record.transaction_id).unwrap().1.in_dispute;

                    if associated_amount.is_some() && tx_in_dispute { //if this tx is not in dispute we can skip all proceeding logic
                        let amount = associated_amount.unwrap(); //safe to unwrap after is_some check
                        
                        if account_hashmap.contains_key(&associated_client_id) {
                    
                            let is_locked = account_hashmap.get_key_value(&associated_client_id).unwrap().1.locked;//safe to unwrap here, we have already done a contains key check    
                            if !is_locked {
                                account_hashmap.entry(associated_client_id).and_modify(|a| {
                                    a.held -= amount;
                                    a.total -= amount;
                                    a.locked = true;
                                });

                                transaction_hashmap.entry(record.transaction_id).and_modify(|t| t.in_dispute = false); //update this transaction as disputed
                            }
                        }
                    }
                }

            }
            
            _ => (), //for all other tx types we assume its an error in the csv and do nothing
            
        }        
        println!("{:?}", record);
    }
    
    Ok(())
}