use std::borrow::Cow;

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

impl std::fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
       write!(f, "{:#?}", self)
    }
}

impl std::error::Error for DatabaseError {}

pub type Result<S> = std::result::Result<S, DatabaseError>;

#[derive(Debug, Clone, PartialEq)]
pub enum MetricValue<'a> {
    Double(f64),
    String(Cow<'a, str>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Metric<'a> {
    pub name: Cow<'a, str>,
    pub when: Cow<'a, DateTime<Utc>>,
    pub value: MetricValue<'a>,
}

pub trait Database: Send + Sync {
    fn setup(&self) -> Result<()>;
    fn write_metric(&self, metric: &Metric) -> Result<()>;
    /// reads metrics with exclusive time ranges
    fn read_metrics<'a>(&'a self, prefix: &str, start: Option<&DateTime<Utc>>, stop: Option<&DateTime<Utc>>)
        -> Result<Vec<Metric<'a>>>;
}