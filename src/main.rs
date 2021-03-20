use tonic::{transport};

mod server;

use crate::server::{
    Server,
    proto::{
        FILE_DESCRIPTOR_SET,
        logger_service_server::LoggerServiceServer,
    },
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse()?;

    // build reflection service
    let reflection_service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
        .build()
        .unwrap();

    let logger_service = LoggerServiceServer::new(Server::default());

    transport::Server::builder()
        .add_service(reflection_service)
        .add_service(logger_service)
        .serve(addr)
        .await?;

    Ok(())
}