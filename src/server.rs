use tonic::{Request, Response, Status};

pub mod proto {
    tonic::include_proto!("logger");
    pub const FILE_DESCRIPTOR_SET: &'static [u8] =
        tonic::include_file_descriptor_set!("logger_descriptor");
}

use proto::{
    RecordMetricsResponse,
    RecordMetricsRequest,
    logger_service_server::LoggerService,
};

#[derive(Debug, Default)]
pub struct Server;

#[tonic::async_trait]
impl LoggerService for Server {
    async fn record_metric(&self, _request: Request<RecordMetricsRequest>)
        -> Result<Response<RecordMetricsResponse>, Status> {
        Ok(Response::new(RecordMetricsResponse{}))
    }
}