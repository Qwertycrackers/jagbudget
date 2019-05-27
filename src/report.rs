
use rusqlite::{Connection, params};
use chrono::naive::NaiveDate;
use chrono::Datelike;
use std::io;
use super::{Budget, Checkpoint};

type ReportError = Box<std::error::Error>;
/// Produce a report on the current state of the budget into `writer`
pub fn report<W: io::Write>(mut w: W, db: &Connection) -> Result<(), ReportError> {
  let budget = db.query_row(
    "SELECT * FROM budget ORDER BY start DESC LIMIT 1;",
    params![],
    |row| Ok(Budget { 
      start: NaiveDate::from_num_days_from_ce(row.get(0)?),
      savings: toml::de::from_slice(&row.get::<usize, Vec<u8>>(1).unwrap()).unwrap(),
      expenditure: toml::de::from_slice(&row.get::<usize, Vec<u8>>(2).unwrap()).unwrap(),
      spend_categories: toml::de::from_slice(&row.get::<usize, Vec<u8>>(3)?).unwrap(),
    })
  )?;
  let checkpoint: Checkpoint = db.query_row(
    "SELECT * FROM checkpoint ORDER BY day DESC LIMIT 1;",
    params![],
    |row| Ok(Checkpoint { assets: row.get(0)?, day: NaiveDate::from_num_days_from_ce(row.get(1)?)})
  )?;
  write!(w, 
    "--BUDGET REPORT--\nYour most recent checkpoint is {}, with assets of ${}.\n", 
    checkpoint.day, checkpoint.assets / 100)?;
  let income_total: u32 = db.query_row(
    "SELECT SUM(amount) FROM income WHERE day >= ? AND day <= date('now');",
    params![checkpoint.day.num_days_from_ce()],
    |row| row.get(0)
  )?;
  let expense_total: u32 = db.query_row(
    "SELECT SUM(cost) FROM expense WHERE day >= ? AND day <= date('now');",
    params![checkpoint.day.num_days_from_ce()],
    |row| row.get(0)
  )?;
  write!(
    w,
    "Since this checkpoint, your income was ${} and you spent ${}.\nThis indicates a savings rate of {}%, and your current account balance should be ${}.\n",
    income_total / 100, expense_total / 100, (income_total - expense_total) * 100 / income_total,
    (checkpoint.assets + income_total - expense_total) / 100)?;
  
  Ok(())
}
