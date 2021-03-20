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
use rust_embed::RustEmbed;

use super::{
    Database,
    DatabaseError,
    Metric,
    MetricValue,
    Result,
    migrator::{
        migrate,
        sqlite::SqliteMigrator,
    },
};

impl From<PoisonError<MutexGuard<'_, Connection>>> for DatabaseError {
    fn from(e: PoisonError<MutexGuard<'_, Connection>>) -> Self {
        DatabaseError::Custom(format!("mutex error: {}", e))
    }
}

impl From<rusqlite::Error> for DatabaseError {
    fn from(e: rusqlite::Error) -> Self {
        DatabaseError::Custom(format!("problem interacting with database: {}", e))
    }
}

#[derive(RustEmbed)]
#[folder = "migrations/sqlite"]
struct Migrations;

pub struct SqliteDatabase{
    conn: Arc<Mutex<Connection>>,
}

impl SqliteDatabase{
    pub fn new(path: &str) -> Result<Self> {
        Ok(SqliteDatabase {
            conn: Arc::new(Mutex::new(Connection::open(path)?)),
        })
    }
}

impl Database for SqliteDatabase {
    fn setup(&self) -> Result<()> {
        Ok(migrate::<Migrations, _>(&SqliteMigrator::new(self.conn.clone()))?)
    }
    fn write_metric(&self, metric: &Metric) -> Result<()> {
        let (typ, dvalue, tvalue) = match metric.value {
            MetricValue::Double(d) => ("double", d, ""),
            MetricValue::String(s) => ("string", 0.0, s),
        };
        self.conn.lock()?.execute("
            INSERT INTO Metrics (name, time, value_type, dvalue, tvalue) VALUES (?1, ?2, ?3, ?4, ?5)
        ", params![metric.name, metric.when.to_rfc3339(), typ, dvalue, tvalue])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use chrono::prelude::*;
    use super::*;

    fn testdb() -> SqliteDatabase {
        let db = SqliteDatabase::new(":memory:").unwrap();
        db.setup().unwrap();
        db
    }

    #[test]
    fn create_database() {
        let db = SqliteDatabase::new(":memory:").unwrap();
        db.setup().unwrap();
    }

    #[test]
    fn insert_value() {
        let db = testdb();
        let date_time = Utc.ymd(2018, 1, 26).and_hms_micro(18, 30, 9, 453_829);
        db.write_metric(&Metric{
            name: "myservice.cpu_time",
            when: &date_time,
            value: MetricValue::Double(23.0),
        }).unwrap();
    }
}