use std::{
    borrow::Cow,
    collections::HashMap,
    time::{UNIX_EPOCH, Duration},
};

use log::{warn};
use chrono::prelude::*;

use tonic::{Request, Response, Status};
use crate::dal::{
    Database,
    DatabaseError,
    Metric,
    MetricValue,
};

pub mod proto {
    tonic::include_proto!("metrics_service");
    pub const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("metrics_service_descriptor");
}

use proto::{
    RecordMetricsResponse,
    RecordMetricsRequest,
    LoadMetricsResponse,
    LoadMetricsRequest,
    metrics_service_server::MetricsService,
    metric::Value as ProtoValue,
    compressed_metric::{
        time_value::Value as CompressedValue,
        TimeValue,
    },
    
};

#[derive(Debug, Default)]
pub struct Server<D: Database> {
    db: D
}

impl<D: Database> Server<D> {
    pub fn new(db: D) -> Self {
        Server{
            db,
        }
    }
}

impl From<DatabaseError> for Status {
    fn from(e: DatabaseError) -> Self {
        warn!("Error accessing database: {}", e);
        Status::internal("trouble accessing database")
    }
}

#[tonic::async_trait]
impl<D: Database + 'static> MetricsService for Server<D> {
    async fn record_metrics(&self, request: Request<RecordMetricsRequest>)
        -> Result<Response<RecordMetricsResponse>, Status> {
        let when = Utc::now();
        for metric in &request.get_ref().metrics {
            let metric_value = match &metric.value {
                Some(ProtoValue::DoubleValue(val)) => MetricValue::Double(*val),
                Some(ProtoValue::StringValue(val)) => MetricValue::String(Cow::Borrowed(val)),
                None => return Err(Status::unknown("Did you ask for a metric with no value? How very foolish of you!")),
            };
            self.db.write_metric(&Metric{
                name: Cow::Borrowed(&metric.identifier),
                when: Cow::Borrowed(&when),
                value: metric_value,
            })?;
        }
        Ok(Response::new(RecordMetricsResponse{}))
    }

    async fn load_metrics(&self, request: Request<LoadMetricsRequest>)
        -> Result<Response<LoadMetricsResponse>, Status> {
        let req = request.get_ref();
        let mut start = None;
        let mut stop = None;
        if let Some(range) = &req.time_range {
            if let Some(start_proto) = &range.start {
                let d = UNIX_EPOCH + Duration::from_secs(start_proto.seconds as u64) + Duration::from_nanos(start_proto.nanos as u64);
                start = Some(DateTime::from(d));
            }
            if let Some(stop_proto) = &range.stop {
                let d = UNIX_EPOCH + Duration::from_secs(stop_proto.seconds as u64) + Duration::from_nanos(stop_proto.nanos as u64);
                stop = Some(DateTime::from(d));
            }
        }
        let mut mapping: HashMap<Cow<'_, str>, Vec<TimeValue>> = HashMap::default();
        for metric in self.db.read_metrics(&req.prefix, start.as_ref(), stop.as_ref())? {
            if !mapping.contains_key(&metric.name) {
                mapping.insert(metric.name.clone(), vec!());
            }
            let value = match metric.value {
                MetricValue::String(v) => CompressedValue::StringValue(v.into_owned()),
                MetricValue::Double(v) => CompressedValue::DoubleValue(v),
            };
            let when = metric.when.into_owned();
            mapping.get_mut(&metric.name).unwrap().push(TimeValue{
                value: Some(value),
                when: Some(prost_types::Timestamp{
                    seconds: when.timestamp(),
                    nanos: when.timestamp_subsec_nanos() as i32,
                }),
            });
        }
        let mut metrics = Vec::with_capacity(mapping.len());
        for (key, value) in mapping {
            metrics.push(proto::CompressedMetric{
                identifier: key.to_string(),
                time_values: value,
            })
        }
        Ok(Response::new(LoadMetricsResponse{metrics}))
    }
}