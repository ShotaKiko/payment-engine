use std::env;
use std::error::Error;
use std::process;
use std::ffi::OsString;
use csv::ReaderBuilder;
use serde::Deserialize;


#[derive(Debug, Deserialize)]
struct TransactionRecord {
    #[serde(rename = "type")]
    transaction_type: String, //TODO: incorporate an enum check on strings to weed out invalid entry types -> csv error
    #[serde(rename = "client")]
    client_id: u16,
    #[serde(rename = "tx")]
    transaction_id: u32,
    #[serde(deserialize_with = "csv::invalid_option")] // in case the below tx types have nulls, some absence of value designation etc.
    amount: Option<f64>, //Option because dispute, resolve and chargeback types have no associated amount
}


fn main() {
    if let Err(error) = run_payment_engine() {
        println!("{}", error);
        process::exit(1);
    }
 }


 
 //TODO: find a mmore specific error type
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

    parse_csv(file_path)?;

    Ok(())
}

//Parse csv into hashmap to run account statement logic
fn parse_csv(file_path: OsString) -> Result<(), Box<dyn Error>> {

    let mut reader = ReaderBuilder::new().trim(csv::Trim::All).from_path(file_path)?; //also trims all whitespace

    for record_result in reader.deserialize() {
        let record: TransactionRecord = record_result?;

        println!("{:?}", record);
    }

    Ok(())
}