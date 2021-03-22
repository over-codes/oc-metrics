use tonic::{transport};
use log::{info};

use oc_metrics::{
    dal::{
        Database,
        sqlite::SqliteDatabase,
    },
    server::{
        Server,
        proto::{
            FILE_DESCRIPTOR_SET,
            metrics_service_server::MetricsServiceServer,
        },
}   ,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // grab env variables
    env_logger::init();
    let dbpath = std::env::var("DBPATH").unwrap_or(":memory:".into());
    let addr = std::env::var("LISTEN").unwrap_or("[::1]:50051".into());
    let addr = addr.parse()?;
    info!("Starting server on port {} with database {}", addr, dbpath);

    // build reflection service
    let reflection_service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
        .build()
        .unwrap();
    
    let db = SqliteDatabase::new(&dbpath)?;
    db.setup()?;

    let logger_service = MetricsServiceServer::new(Server::new(db));

    transport::Server::builder()
        .add_service(reflection_service)
        .add_service(logger_service)
        .serve(addr)
        .await?;

    Ok(())
}