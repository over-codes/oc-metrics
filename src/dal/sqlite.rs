use std::{
    borrow::Cow,
    sync::{
        Arc,
        Mutex,
        MutexGuard,
        PoisonError,
    },
};

use chrono::prelude::*;
use rusqlite::{
    Connection,
    ToSql,
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

impl From<chrono::ParseError> for DatabaseError {
    fn from(e: chrono::ParseError) -> Self {
        DatabaseError::Custom(format!("problem parsing timestamp from database: {}", e))
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
        let (typ, dvalue, tvalue) = match &metric.value {
            MetricValue::Double(d) => ("double", *d, Cow::Borrowed("")),
            MetricValue::String(s) => ("string", 0.0, s.clone()),
        };
        self.conn.lock()?.execute("
            INSERT INTO Metrics (name, time, value_type, dvalue, tvalue) VALUES (?1, ?2, ?3, ?4, ?5)
        ", params![metric.name, metric.when.to_rfc3339(), typ, dvalue, tvalue])?;
        Ok(())
    }

    fn read_metrics<'a>(&'a self, prefix: &str, start: Option<&DateTime<Utc>>, stop: Option<&DateTime<Utc>>)
        -> Result<Vec<Metric<'a>>> {
        // prepare the query
        let mut query = "
            SELECT t1.name,
                t1.time,
                t1.value_type,
                t1.dvalue,
                t1.tvalue
            FROM Metrics t1
            WHERE t1.name LIKE :prefix
        ".to_string();
        let start_string;
        let stop_string;
        let prefix = &format!("{}%", prefix);
        let mut params: Vec<(&str, &dyn ToSql)> = vec!((":prefix", &prefix));
        if let Some(start) = start {
            query += "
                AND t1.time > :start
            ";
            start_string = start.to_rfc3339();
            params.push((":start", &start_string));
        };
        if let Some(stop) = stop {
            query += "
                AND t1.time < :stop
            ";
            stop_string = stop.to_rfc3339();
            params.push((":stop", &stop_string));
        };
        let conn = self.conn.lock()?;
        let mut stmt = conn.prepare(&query)?;
        let mut rows = stmt.query_named(params.as_slice())?;
        let mut metrics = vec!();
        while let Some(row) = rows.next()? {
            let date_time:String = row.get(1)?;
            let date_time: DateTime<Utc> = DateTime::parse_from_rfc3339(&date_time)?.with_timezone(&Utc);
            let typ: String = row.get(2)?;
            let value = if typ == "double"{
                MetricValue::Double(row.get(3)?)
            } else {
                MetricValue::String(Cow::Owned(row.get(4)?))
            };
            metrics.push(Metric{
                name: Cow::Owned(row.get(0)?),
                when: Cow::Owned(date_time),
                value,
            });
        }
        Ok(metrics)
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
            name: Cow::Borrowed("myservice.cpu_time"),
            when: Cow::Owned(date_time),
            value: MetricValue::Double(23.0),
        }).unwrap();
    }

    #[test]
    fn load_values() {
        let db = testdb();
        let date_time = Utc.ymd(2018, 1, 26).and_hms_micro(18, 30, 9, 453_829);
        let metric = Metric{
            name: Cow::Borrowed("myservice.cpu_time"),
            when: Cow::Owned(date_time),
            value: MetricValue::Double(23.0),
        };
        db.write_metric(&metric).unwrap();
        let got_metrics = db.read_metrics("myservice.", None, None).unwrap();
        assert_eq!(
            got_metrics,
            vec!(metric),
        )
    }

    #[test]
    fn load_values_with_timerange() {
        let db = testdb();
        let before = Utc.ymd(2018, 1, 26).and_hms_micro(18, 30, 9, 453_829);
        let valid = Utc.ymd(2019, 1, 26).and_hms_micro(18, 30, 9, 453_829);
        let after =  Utc.ymd(2020, 1, 26).and_hms_micro(18, 30, 9, 453_829);
        let mut want_metrics = vec!();
        for date_time in vec!(before, valid, after) {
            let metric = Metric{
                name: Cow::Borrowed("myservice.cpu_time"),
                when: Cow::Owned(date_time),
                value: MetricValue::Double(23.0),
            };
            db.write_metric(&metric).unwrap();
            want_metrics.push(metric);
        }
        let got_metrics = db.read_metrics("myservice.", Some(&before), Some(&after)).unwrap();
        assert_eq!(
            got_metrics,
            vec!(want_metrics[1].clone()),
        )
    }
}