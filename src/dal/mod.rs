use chrono::prelude::*;


pub mod migrator;
pub mod sqlite;

#[derive(Debug, Clone)]
pub enum DatabaseError{
    Custom(String),
    MigrationError(migrator::MigrationError),
}

impl From<migrator::MigrationError> for DatabaseError {
    fn from(e: migrator::MigrationError) -> Self {
        DatabaseError::MigrationError(e)
    }
}

pub type Result<S> = std::result::Result<S, DatabaseError>;

pub enum MetricValue<'a> {
    Double(f64),
    String(&'a str),
}

pub struct Metric<'a> {
    pub name: &'a str,
    pub when: &'a DateTime<Utc>,
    pub value: MetricValue<'a>,
}

pub trait Database {
    fn setup(&self) -> Result<()>;
    fn write_metric(&self, metric: &Metric) -> Result<()>;
}