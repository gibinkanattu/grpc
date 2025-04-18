use tonic::{transport::Channel, Request};
use dataserver::{Data, Acknowledgement};
use tokio_stream::{StreamExt,Stream};
use dataserver::data_transmission_client::DataTransmissionClient;


pub mod dataserver {
    tonic::include_proto!("dataserver");
}

fn echo_requests_iter() -> impl Stream<Item = Data> {
    tokio_stream::iter(1..4).map(|i| Data {
        request_msg: format!("msg {:02}", i),
    })
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = DataTransmissionClient::connect("http://[::1]:50051").await?;

    // Unary call
    let request = tonic::Request::new(Data {
        request_msg: "UnaryUser".into(),
    });
    let response = client.unary_server(request).await?;
    println!("Unary Response: {:?}", response.into_inner().message);

    // Server streaming
    let request = tonic::Request::new(Data {
        request_msg: "StreamUser".into(),
    });
    let mut stream = client.server_streaming(request).await?.into_inner();
    println!("Server Streaming:");
    while let Some(reply) = stream.message().await? {
        println!("  -> {}", reply.request_msg);
    }

    // Client streaming
    let in_stream = echo_requests_iter().take(4);
    let response = client
        .client_streaming(in_stream)
        .await
        .unwrap();
    println!("Client Streaming Response: {}", response.into_inner().request_msg);

    // Bidirectional streaming
    println!("Chat Streaming Response");
    let in_stream = echo_requests_iter().take(4);
    let response = client
        .chat_streaming(in_stream)
        .await
        .unwrap();
    let mut resp_stream = response.into_inner();

    while let Some(received) = resp_stream.next().await {
        let received = received.unwrap();
        println!("\treceived message: `{}`", received.request_msg);
    }
    Ok(())
}