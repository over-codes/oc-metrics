use tonic::{transport};

use rust_grpc_test::{
    dal::{
        Database,
        sqlite::SqliteDatabase,
    },
    server::{
        Server,
        proto::{
            FILE_DESCRIPTOR_SET,
            logger_service_server::LoggerServiceServer,
        },
}   ,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // grab env variables
    env_logger::init();
    let dbpath = std::env::var("DBPATH").unwrap_or(":memory:".to_string());
    let addr = "[::1]:50051".parse()?;

    // build reflection service
    let reflection_service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
        .build()
        .unwrap();
    
    let db = SqliteDatabase::new(&dbpath)?;
    db.setup()?;

    let logger_service = LoggerServiceServer::new(Server::new(db));

    transport::Server::builder()
        .add_service(reflection_service)
        .add_service(logger_service)
        .serve(addr)
        .await?;

    Ok(())
}