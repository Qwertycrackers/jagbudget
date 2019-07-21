extern crate serde;
extern crate serde_derive;
extern crate toml;
extern crate clap;
extern crate chrono;
extern crate comp;

use chrono::naive::{NaiveDate};
use chrono::Datelike;
use serde_derive::{Serialize, Deserialize};
use clap::{Arg, App};
use std::fs::File;
use std::io;
use std::collections::HashMap;

mod report;
mod utils;

type BoxError = Box<std::error::Error>;

#[derive(Serialize, Deserialize, Debug)]
struct Expense {
  amount: u32,
  category: String,
  detail: String,
  day: NaiveDate,
}

#[derive(Deserialize)]
struct Income {
  income: u32,
  category: String,
  day: NaiveDate,
}

/// Configuration of budgetary targets for a time period.
#[derive(Deserialize)]
struct Budget {
  start: NaiveDate,
  /// The target proportion and flat amounts of savings
  savings: Alloc,
  /// The target proportion and flat amounts of spending
  expenditure: Alloc,
  /// Spending allocations by category
  spend_categories: HashMap<String, Alloc>,
}

/// An allocation of money.
#[derive(Serialize, Deserialize)]
struct Alloc {
  /// This allocation as a proportion. Min or max based on context.
  rate: f32,
  /// This allocation as a flat value in cents. Min or max based on context.
  flat: u32,
}

/// Checkpoint re-sets the exact quantity of liquid assets at a single time, to handle innaccurate reporting.
#[derive(Deserialize)]
pub struct Checkpoint {
  assets: u32,
  day: NaiveDate,
}

fn main() -> Result<(), BoxError> {
  // Parse args
  let matches = App::new("JAG Budget")
    .version(option_env!("CARGO_PKG_VERSION").unwrap_or("unknown"))
    .author("Joseph A. Gerardot <joseph@gerardot.org>")
    .about("Ingests weakly-structured income and expense records for analysis.")
    .arg(Arg::with_name("files")
         .short("f")
         .multiple(true)
         .value_delimiter(" ")
         .help("List of the file names that contain inputs.")
         .takes_value(true)
    )
    .arg(Arg::with_name("database")
         .short("d")
         .help("The SQLite file from which to read and write data. If this does not exist it will be created.")
         .takes_value(true)
    )
    .get_matches();
  // Our workload is ultimately an SQL-y workload, so we're just going to use SQLite off the bat.
  let db = matches.value_of_os("database");
  Ok(())
}

