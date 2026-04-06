use read_config::read_certs;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::codec::CompressionEncoding;
use tonic::service::interceptor::{self, InterceptedService};
use tonic::{
    Request, Response, Status,
    metadata::MetadataValue,
    transport::{
        Identity, Server, ServerTlsConfig,
        server::{TcpConnectInfo, TlsConnectInfo},
    },
};
use tower_http::validate_request::ValidateRequestHeaderLayer;
mod read_config;
pub mod dataserver {
    tonic::include_proto!("dataserver");
}
use clap::Parser;
use dataserver::data_transmission_server::{DataTransmission, DataTransmissionServer};
use dataserver::{Acknowledgement, Data};
// defining a struct for our service
#[derive(Default)]
pub struct MyDataTransmission;

// implementing rpc for service defined in .proto
#[tonic::async_trait]
impl DataTransmission for MyDataTransmission {
    // unary rpc impelemented as function
    async fn unary_server(
        &self,
        request: Request<Data>,
    ) -> Result<Response<Acknowledgement>, Status> {
        println!("Recieved Unary Request {:?}", request.get_ref().request_msg);
        // returning a response as Data message as defined in .proto
        Ok(Response::new(Acknowledgement {
            status: true,
            // reading data from request which is awrapper around our SayRequest message defined in .proto
            message: format!("Received MSG {}", request.get_ref().request_msg),
        }))
    }

    // Server Streaming RPC
    type ServerStreamingStream = ReceiverStream<Result<Data, Status>>;
    async fn server_streaming(
        &self,
        request: Request<Data>,
    ) -> Result<Response<Self::ServerStreamingStream>, Status> {
        let msg = request.into_inner().request_msg;
        println!("Recieved Server Streaming Request {:?}", msg);
        // define no.of messages to be streamed to client
        let (tx, rx) = mpsc::channel(4);

        // created asynchronous threads to send response
        tokio::spawn(async move {
            for i in 1..=5 {
                let msg = Data {
                    request_msg: format!("Hello {} - {}", msg, i),
                };
                if tx.send(Ok(msg)).await.is_err() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    // Client streaming
    async fn client_streaming(
        &self,
        request: Request<tonic::Streaming<Data>>,
    ) -> Result<Response<Data>, Status> {
        let mut stream = request.into_inner();
        let mut msgs = vec![];

        while let Some(req) = stream.message().await? {
            println!("Recieved Client Streaming Request {:?}", req.request_msg);
            msgs.push(req.request_msg);
        }

        let reply = Data {
            request_msg: format!("Messages Received {}!", msgs.join(", ")),
        };

        Ok(Response::new(reply))
    }

    // Bidirectional streaming
    type ChatStreamingStream = ReceiverStream<Result<Data, Status>>;

    async fn chat_streaming(
        &self,
        request: Request<tonic::Streaming<Data>>,
    ) -> Result<Response<Self::ChatStreamingStream>, Status> {
        let mut stream = request.into_inner();
        let (tx, rx) = mpsc::channel(4);
        println!("Chat Streaming Received");
        tokio::spawn(async move {
            while let Some(req) = stream.message().await.unwrap_or(None) {
                println!("Recieved Chat Streaming Request {:?}", req.request_msg);
                let msg = Data {
                    request_msg: format!("Recieved {}!", req.request_msg),
                };
                if tx.send(Ok(msg)).await.is_err() {
                    break;
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

// Checking Authentication with interceptors Meta Data
pub fn check_auth(req: Request<()>) -> Result<Request<()>, Status> {
    let token: MetadataValue<_> = "secret_value"
        .parse()
        .expect("Expected MetadataValue to be str");

    match req.metadata().get("secret_key") {
        Some(t) if token == t => Ok(req),
        _ => Err(Status::unauthenticated("No valid auth secret")),
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    file: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let config = read_config::read_config(args.file);
    let server_address = config["server_address"]
        .as_str()
        .unwrap_or("0.0.0.0:50051")
        .parse()?;

    // check if security needs to be implemented
    let tls_enabled = config["authentication_enabled"].as_bool().ok_or("authentication_enabled option not configured correctly in config file!!, Type expected boolean.")?;

    let result = if tls_enabled == true {
        // If security enabled -> Add TLS, JWT Token and secret key-value pair authentications
        let cert = read_certs(config.get("server-cert").unwrap().to_string().replace("\"", "")).unwrap();
        let key = read_certs(config.get("server-key").unwrap().to_string().replace("\"", "")).unwrap();
        let identity = Identity::from_pem(cert, key);
        let jwt_token = config.get("token").unwrap().to_string().replace("\"", "");

        // Adding compression to the service
        let svc = DataTransmissionServer::new(MyDataTransmission)
            .accept_compressed(CompressionEncoding::Gzip)
            .send_compressed(CompressionEncoding::Gzip);
        // Adding Interceptor to service
        let svc_compressed = InterceptedService::new(svc, check_auth);

        println!("Building server with TLS and security {:?}",server_address);
        // Build server
        Server::builder()
            .max_concurrent_streams(1000) // max concurrent streams
            .concurrency_limit_per_connection(100) //the maximum number of connections allowed per client
            .max_frame_size(1 * 1024 * 1024) // the maximum frame size 1 Mb
            .tls_config(ServerTlsConfig::new().identity(identity))?
            .layer(ValidateRequestHeaderLayer::bearer(&jwt_token))
            .add_service(svc_compressed)
            .serve(server_address)
            .await?;
        Ok(())
    } else {
        let data_transmission = MyDataTransmission::default();
        let svc = DataTransmissionServer::new(data_transmission)
            .accept_compressed(CompressionEncoding::Gzip)
            .send_compressed(CompressionEncoding::Gzip);
        println!("Building server without TLS and security {:?}",server_address);
        Server::builder()
            .add_service(svc)
            .serve(server_address)
            .await?;
        Ok(())
    };

    result
}
