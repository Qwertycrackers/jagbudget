extern crate serde;
extern crate serde_derive;
extern crate toml;
extern crate clap;
extern crate rusqlite;
extern crate chrono;

use chrono::naive::{NaiveDate};
use serde::{Serialize, Deserialize};
use serde_derive::{Serialize, Deserialize};
use clap::{Arg, App, SubCommand};
use rusqlite::{Connection};
use std::fs::File;
use std::io;
use std::collections::HashMap;

type BoxError = Box<std::error::Error>;

#[derive(Deserialize)]
struct Expense {
  amount: u32,
  category: String,
  detail: String,
  day: NaiveDate,
}

#[derive(Deserialize)]
struct Income {
  amount: u32,
  category: String,
  day: NaiveDate,
}

/// Configuration of budgetary targets for a time period.
#[derive(Deserialize)]
struct Budget {
  start: NaiveDate,
  end: NaiveDate,
  /// The target proportion and flat amounts of savings
  savings: Alloc,
  /// The target proportion and flat amounts of spending
  expenditure: Alloc,
  /// Spending allocations by category
  spend_categories: Option<HashMap<String, Alloc>>,
}

/// An allocation of money.
#[derive(Serialize, Deserialize)]
struct Alloc {
  /// This allocation as a proportion. Min or max based on context.
  rate: f32,
  /// This allocation as a flat value in cents. Min or max based on context.
  flat: u32,
}

fn main() -> Result<(), BoxError> {
  // Parse args
  let matches = App::new("JAG Budget")
    .version(option_env!("CARGO_PKG_VERSION").unwrap_or("unknown"))
    .author("Joseph A. Gerardot <joseph@gerardot.org>")
    .about("Ingests weakly-structured income and expense records for analysis.")
    .arg(Arg::with_name("files")
         .short("f")
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
  let db = matches.value_of_os("database")
    .map(Connection::open)
    .unwrap_or_else(Connection::open_in_memory)?;
  rectify_db(&db);
  // Ingest expense records
  let file_names = matches.values_of_os("files");
  if let Some(file_names) = file_names {
    file_names.for_each( |f| {
      let file = File::open(f);
      match file.map(|x| parse_into_sqlite( x, &db)) {
        Err(_) => eprint!("File was badly-formed or couldn't be opened: {}", f.to_string_lossy()),
        _ => (), // No need to print an error
      }
    });
  }
  // TODO: Produce report 
  Ok(())
}

fn rectify_db(db: &Connection) -> () {
  if !is_db_correct(db) { init_db(&db); }
}

fn init_db(db: &Connection) -> () {
  db.execute_batch("
    CREATE TABLE expense (
      cost INTEGER,
      category TEXT,
      detail TEXT,
      day DATE
    );
    CREATE TABLE income (
      amount INTEGER,
      category TEXT,
      day DATE
    );
    CREATE TABLE budget (
      start DATE,
      end DATE,
      savings BLOB,
      expenditure BLOB,
      spend_categories BLOB
    );
  ").unwrap();
}

fn is_db_correct(db: &Connection) -> bool {
  false // Always re-initialize db
}

fn parse_into_sqlite<R: io::Read>(mut file: R, db: &Connection) -> Result<(), toml::de::Error> {
  let mut bytes = Vec::new();
  file.read_to_end(&mut bytes).unwrap();
  toml::de::from_slice::<Expense>(&bytes)
    .map(|expense| expense.insert_sql(db))
    .or_else(|_| {
      toml::de::from_slice::<Income>(&bytes)
        .map(|income| income.insert_sql(db))
        .or_else(|_| {
          toml::de::from_slice::<Budget>(&bytes)
            .map(|budget| budget.insert_sql(db))
        })
    })
    .map(|_| ())
}

trait InsertSql {
  fn insert_sql(&self, db: &Connection) -> Result<(), BoxError>;
}

impl InsertSql for Expense {
  fn insert_sql(&self, db: &Connection) -> Result<(), BoxError> {
    db.execute(
      "INSERT INTO expense VALUE (?, ?, ?, ?);",
      &[self.amount, &self.category, &self.detail, self.day]
    ).map(|_| ())
  }
}

impl InsertSql for Income {
  fn insert_sql(&self, db: &Connection) -> Result<(), BoxError> {
    db.execute(
      "INSERT INTO income VALUE (?, ?, ?, ?);",
      &[self.amount, &self.category, self.day]
    ).map(|_| ())
  }
}

impl InsertSql for Budget {
  fn insert_sql(&self, db: &Connection) -> Result<(), BoxError> {
    db.execute(
      "INSERT INTO budget VALUE (?, ?, ?, ?);",
      &[self.start, 
        self.end, 
        toml::ser::to_vec(&self.savings).unwrap(),
        toml::ser::to_vec(&self.expenditure).unwrap(),
        toml::ser::to_vec(&self.spend_categories).unwrap(),
      ]
    ).map(|_| ())
  }
}
