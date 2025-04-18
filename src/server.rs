use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::codec::CompressionEncoding;
use tonic::{Request, Response, Status, transport::Server};
use tower_http::validate_request::ValidateRequestHeaderLayer;
pub mod dataserver {
    tonic::include_proto!("dataserver");
}
use dataserver::data_transmission_server::{DataTransmission, DataTransmissionServer};
use dataserver::{Acknowledgement, Data};

// defining a struct for our service
#[derive(Default)]
pub struct MyDataTransmission {}

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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // defining address for our service
    let addr = "[::1]:50051".parse().unwrap();
    // creating a service
    let say = MyDataTransmission::default();
    println!("Server listening on {}", addr);
    // adding our service to our server.
    Server::builder()
        .add_service(DataTransmissionServer::new(say))
        .serve(addr)
        .await?;
    Ok(())
}
