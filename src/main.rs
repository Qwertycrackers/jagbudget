extern crate serde;
extern crate serde_derive;
extern crate toml;
extern crate clap;
extern crate rusqlite;
extern crate chrono;
extern crate comp;

use chrono::naive::{NaiveDate};
use chrono::Datelike;
use serde_derive::{Serialize, Deserialize};
use clap::{Arg, App};
use rusqlite::{Connection, params};
use std::fs::File;
use std::io;
use std::collections::HashMap;

mod report;

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
        Err(e) => eprint!("File {} couldn't be opened!\n", f.to_string_lossy()),
        Ok(r) => match r {
          Ok(()) => (), 
          Err(_) => eprint!("File '{}' did not parse or insert correctly! Reason: {:?}\n",
                            f.to_string_lossy(), e),
        },
      }
    });
  }
  report::report(io::stdout().lock(), &db)
}

fn rectify_db(db: &Connection) -> () {
  if !is_db_correct(db) { init_db(&db); }
}

fn init_db(db: &Connection) -> () {
  db.execute_batch("
    DROP TABLE IF EXISTS expense;
    CREATE TABLE expense (
      cost INTEGER,
      category TEXT,
      detail TEXT,
      day DATE
    );
    CREATE INDEX IF NOT EXISTS expense_day ON expense (day DESC);
    DROP TABLE IF EXISTS income;
    CREATE TABLE income (
      amount INTEGER,
      category TEXT,
      day DATE
    );
    CREATE INDEX IF NOT EXISTS income_day ON income (day DESC);
    DROP TABLE IF EXISTS budget;
    CREATE TABLE budget (
      start DATE,
      savings BLOB,
      expenditure BLOB,
      spend_categories BLOB
    );
    CREATE INDEX IF NOT EXISTS budget_start ON budget (start DESC);
    DROP TABLE IF EXISTS checkpoint;
    CREATE TABLE checkpoint (
      assets INTEGER,
      day DATE
    );
    CREATE INDEX IF NOT EXISTS checkpoint_day ON checkpoint (day DESC);
  ").unwrap();
}

fn is_db_correct(db: &Connection) -> bool {
  let budget = db.query_row(
    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'budget';",
    params![],
    |row| row.get::<usize, i32>(0).map(|count| count > 0)
  ).unwrap_or(false);
  let income = db.query_row(
    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'income';",
    params![],
    |row| row.get::<usize, i32>(0).map(|count| count > 0)
  ).unwrap_or(false);
  let expense = db.query_row(
    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'expense';",
    params![],
    |row| row.get::<usize, i32>(0).map(|count| count > 0)
  ).unwrap_or(false);
  let checkpoint = db.query_row(
    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'checkpoint';",
    params![],
    |row| row.get::<usize, i32>(0).map(|count| count > 0)
  ).unwrap_or(false);

  expense && income && budget && checkpoint
}

fn parse_into_sqlite<R: io::Read>(mut file: R, db: &Connection) -> Result<(), toml::de::Error> {
  let mut bytes = Vec::new();
  file.read_to_end(&mut bytes).unwrap();
  toml::de::from_slice::<Expense>(&bytes)
    .map(|expense| expense.insert_sql(db).unwrap())
    .or_else(|_| {
      toml::de::from_slice::<Income>(&bytes)
        .map(|income| income.insert_sql(db).unwrap())
        .or_else(|_| {
          toml::de::from_slice::<Budget>(&bytes)
            .map(|budget| budget.insert_sql(db).unwrap())
            .or_else(|_| {
              toml::de::from_slice::<Checkpoint>(&bytes)
                .map(|checkpoint| checkpoint.insert_sql(db).unwrap())
            })
        })
    })
    .map(|_| ())
}

trait InsertSql {
  fn insert_sql(&self, db: &Connection) -> Result<(), rusqlite::Error>;
}

impl InsertSql for Expense {
  fn insert_sql(&self, db: &Connection) -> Result<(), rusqlite::Error> {
    db.execute(
      "INSERT INTO expense VALUES (?, ?, ?, ?);",
      params![self.amount, &self.category, &self.detail, self.day.num_days_from_ce()]
    ).map(|_| ())
  }
}

impl InsertSql for Income {
  fn insert_sql(&self, db: &Connection) -> Result<(), rusqlite::Error> {
    db.execute(
      "INSERT INTO income VALUES (?, ?, ?);",
      params![self.income, &self.category, self.day.num_days_from_ce()]
    ).map(|_| ())
  }
}

impl InsertSql for Budget {
  fn insert_sql(&self, db: &Connection) -> Result<(), rusqlite::Error> {
    db.execute(
      "INSERT INTO budget VALUES (?, ?, ?, ?);",
      params![
        self.start.num_days_from_ce(), 
        toml::ser::to_vec(&self.savings).unwrap(),
        toml::ser::to_vec(&self.expenditure).unwrap(),
        toml::ser::to_vec(&self.spend_categories).unwrap(),
      ]
    ).map(|_| ())
  }
}

impl InsertSql for Checkpoint {
  fn insert_sql(&self, db: &Connection) -> Result<(), rusqlite::Error> {
    db.execute(
      "INSERT INTO checkpoint VALUES (?, ?)",
      params![self.assets, self.day.num_days_from_ce()]
    ).map(|_| ())
  }
}
