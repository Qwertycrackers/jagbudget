
use diesel::prelude::*;
use chrono::naive::NaiveDate;
use chrono::Datelike;
use std::io;
use super::{Budget, Checkpoint};

type ReportError = Box<std::error::Error>;
/// Produce a report on the current state of the budget into `writer`
pub fn report<W: io::Write>(mut w: W, conn: &SqliteConnection) -> Result<(), ReportError> {

}
