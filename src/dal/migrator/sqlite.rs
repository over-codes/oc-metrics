use std::{
    sync::{
        Arc,
        Mutex,
        MutexGuard,
        PoisonError,
    },
};
use rusqlite::{
    Connection,
    params,
};

use super::{
    Applier,
    Result,
    MigrationError,
};

impl From<PoisonError<MutexGuard<'_, Connection>>> for MigrationError {
    fn from(e: PoisonError<MutexGuard<'_, Connection>>) -> Self {
        MigrationError(format!("mutex error: {}", e))
    }
}

impl From<rusqlite::Error> for MigrationError {
    fn from(e: rusqlite::Error) -> Self {
        MigrationError(format!("problem interacting with database: {}", e))
    }
}

#[derive(Clone)]
pub struct SqliteMigrator {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteMigrator {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        SqliteMigrator{conn}
    }
}


impl Applier for SqliteMigrator {
    /// sets up the migration table; this should be idempotent
    fn setup(&self) -> Result<()> {
        self.conn.lock()?.execute_batch("
            CREATE TABLE IF NOT EXISTS SchemaMigrations (
                name TEXT PRIMARY KEY
            );
        ")?;
        Ok(())
    }

    /// applies a schema-altering SQL statement
    fn apply(&self, sql: &str) -> Result<()> {
        self.conn.lock()?.execute_batch(sql)?;
        Ok(())
    }

    /// mark_applied marks the migration as applied
    fn mark_applied(&self, name: &str) -> Result<()> {
        self.conn.lock()?.execute("
            INSERT INTO SchemaMigrations (name) VALUES (?1)
        ", params![name])?;
        Ok(())
    }

    /// retrieves all applied migrations
    fn applied(&self) -> Result<Vec<String>> {
        let conn = self.conn.lock()?;
        let mut stmt = conn.prepare("
            SELECT t1.name
            FROM SchemaMigrations t1
        ")?;
        let mut rows = stmt.query(params![])?;
        let mut names = vec!();
        while let Some(row) = rows.next()? {
            names.push(row.get(0)?);
        }
        Ok(names)
    }
}