use std::env;
use std::error::Error;
use std::process;
use std::ffi::OsString;



fn main() {
    if let Err(error) = run_payment_engine() {
        println!("{}", error);
        process::exit(1);
    }
 }


//Pull file path from user input returning an error if a file path argument is not supplied
//TODO: find a mmore specific error type

fn get_file_path() -> Result<OsString, Box<dyn Error>> {
    match env::args_os().nth(1) {
        None => Err(From::from("expected 1 arguments(the csv file to read from), but got none")),
        Some(file_path) => Ok(file_path),
    }
}


fn run_payment_engine() -> Result<(), Box<dyn Error>> {
    let file_path = get_file_path()?; //Add a check for file path type (ie: confirm it is a .csv)

    let mut reader = csv::Reader::from_path(file_path)?;

    for record_result in reader.records() {
        let record = record_result?;

        println!("{:?}", record);
    }

    Ok(())
}