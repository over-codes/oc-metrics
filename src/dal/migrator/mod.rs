//! A super simple database migration manager; create a folder
//! in your project to store your migrations (they will be
//! sorted alphabetically before applying, so I suggest prefacing
//! them with the date or a number like `0001`), include them
//! with RustEmbed, and call the setup script when connecting to
//! your database.
//! 
//! ```
//! use std::{
//!     sync::{
//!         Arc,
//!         Mutex,
//!     },
//! };
//! use rust_embed::RustEmbed;
//! use rusqlite::Connection;
//! 
//! // Include a migrator implementation from this module;
//! // we use sqlite here
//! use oc_metrics::dal::migrator::{
//!     migrate,
//!     sqlite::SqliteMigrator,
//! };
//! 
//! // Create the embedded code to use for migrations
//! #[derive(RustEmbed)]
//! #[folder = "testdata/sqlite"]
//! struct TestData;
//! 
//! // Connect to your database
//! let conn = Arc::new(Mutex::new(Connection::open(":memory:").unwrap()));
//! // Create your applier
//! let applier = SqliteMigrator::new(conn);
//! // Migrate!
//! migrate::<TestData, _>(&applier).unwrap();
//! ```
use std::{
    borrow::Cow,
};

pub mod sqlite;

use rust_embed::RustEmbed;

#[derive(Debug, Clone)]
pub struct MigrationError(String);

pub type Result<T> = std::result::Result<T, MigrationError>;

pub trait Applier {
    /// sets up the migration table; this should be idempotent
    fn setup(&self) -> Result<()>;
    /// applies a schema-altering SQL statement
    fn apply(&self, sql: &str) -> Result<()>;
    /// mark_applied marks the migration as applied
    fn mark_applied(&self, name: &str) -> Result<()>;
    /// retrieves all applied migrations
    fn applied(&self) -> Result<Vec<String>>;
}

pub fn migrate<E: RustEmbed, A: Applier>(applier: &A) -> Result<()> {
    let mut files: Vec<Cow<'static, str>> = E::iter().collect();
    files.sort();

    // make sure the migration schema exists
    applier.setup()?;

    // get the existing migrations
    let mut applied_migrations = applier.applied()?;
    applied_migrations.sort();

    // apply migrations
    let mut i = 0;
    while i < files.len() {
        if i < applied_migrations.len() {
            // we expect the files to match; if not, error out
            if applied_migrations[i] != files[i] {
                return Err(MigrationError(format!(
                    "Problem applying migrations; expected to find applied migration '{}', but found '{}'",
                    applied_migrations[i],
                    files[i])))
            }
        } else {
            // we are applying this migration!
            let raw = E::get(&files[i]).ok_or(MigrationError(format!(
                "Expected to find file {} in embedded files; did not",
                files[i],
            )))?;
            let sql = match std::str::from_utf8(&raw) {
                Ok(s) => s,
                Err(e) => return Err(MigrationError(format!(
                    "Migrations contains poorly formed text: {}",
                    e,
                ))),
            };
            applier.apply(sql)?;
            // If that succeeded, mark the migration as applied
            applier.mark_applied(&files[i])?;
        }
        i += 1;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{
            Arc,
            Mutex,
        },
    };
    use rust_embed::RustEmbed;
    use rusqlite::{
        Connection,
        params,
        NO_PARAMS,
    };

    use super::{
        migrate,
        sqlite::SqliteMigrator,
    };

    #[derive(RustEmbed)]
    #[folder = "testdata/sqlite"]
    struct TestData;

    #[test]
    fn test_apply_new_migrations() {
        let conn = Arc::new(Mutex::new(Connection::open(":memory:").unwrap()));
        let applier = &SqliteMigrator::new(conn.clone());
        migrate::<TestData, _>(applier).unwrap();
        // validate the table exists
        conn.lock().unwrap().execute("
            INSERT INTO Posts (Id) VALUES (?1)
        ", params!["hello world"]).unwrap();
    }

    #[test]
    fn test_reapply_migrations() {
        let conn = Arc::new(Mutex::new(Connection::open(":memory:").unwrap()));
        let applier = &SqliteMigrator::new(conn.clone());
        let want_result = "hello world";
        migrate::<TestData, _>(applier).unwrap();
        // insert a row
        conn.lock().unwrap().execute("
            INSERT INTO Posts (Id) VALUES (?1)
        ", params![want_result]).unwrap();
        migrate::<TestData, _>(applier).unwrap();
        // get that row back
        let got_result = conn.lock().unwrap()
            .query_row("SELECT Id from Posts", NO_PARAMS, |f| {
                Ok(f.get::<usize, String>(0)?)
            }).unwrap();
        assert_eq!(got_result, want_result)
    }
}