extern crate serde;
extern crate serde_derive;
extern crate toml;
extern crate clap;
extern crate rusqlite;

use serde::{Serialize, Deserialize};
use serde_derive::{Serialize, Deserialize};
use clap::{Arg, App, SubCommand};
use rusqlite::{Connection};
use std::fs::File;
use std::io;

fn main() -> Result<(), Box<std::error::Error>> {
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
    file_names.for_each(|f| parse_into_sqlite( File::open(f).unwrap(), &db));
  }
  // TODO: Produce report 
  Ok(())
}

fn rectify_db(db: &Connection) -> () {
  if !is_db_correct(db) { init_db(&db); }
}

fn init_db(db: &Connection) -> () {

}

fn is_db_correct(db: &Connection) -> () {

}

fn parse_into_sqlite<R: io::Read>( file: R, db: &Connection) -> Result<(), Box<std::error::Error>> {

}


