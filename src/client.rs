use std::time::Duration;
use dataserver::data_transmission_client::DataTransmissionClient;
use dataserver::Data;
use read_config::read_certs;
use tokio_stream::{Stream, StreamExt};
use tonic::metadata::MetadataKey;
use tonic::{
    Request, Status,
    codegen::InterceptedService,
    service::Interceptor,
    transport::{Certificate, Channel, ClientTlsConfig},
};
mod read_config;
use clap::Parser;

pub mod dataserver {
    tonic::include_proto!("dataserver");
}

fn echo_requests_iter() -> impl Stream<Item = Data> {
    tokio_stream::iter(1..4).map(|i| Data {
        request_msg: format!("msg {:02}", i),
    })
}
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    file: String,
}

struct AuthInterceptor {
    token: String,
    secret_key: String,
    secret_value: String,
}

impl Interceptor for AuthInterceptor {
    fn call(&mut self, mut req: Request<()>) -> Result<Request<()>, Status> {
        req.metadata_mut()
            .insert("authorization", self.token.parse().unwrap());
        let key = MetadataKey::from_bytes(self.secret_key.as_bytes())
            .map_err(|e| Status::internal(format!("Invalid metadata key: {}", e)))?;
        let value = self.secret_value.parse()
            .map_err(|e| Status::internal(format!("Invalid metadata value: {}", e)))?;
        req.metadata_mut().insert(
            key,
            value,
        );
        Ok(req)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let config = read_config::read_config(args.file);
    let tls_enabled = config["authentication_enabled"].as_bool().ok_or("authentication_enabled option not configured correctly in config file!!, Type expected boolean.")?;
    if tls_enabled == true {
        let pem = read_certs(
            config
                .get("tls_cert")
                .unwrap()
                .to_string()
                .replace("\"", ""),
        )
        .unwrap();
        let ca = Certificate::from_pem(pem);
        let tls = ClientTlsConfig::new()
            .ca_certificate(ca)
            .domain_name(config["domain_name"].to_string().replace("\"", ""));
        // static server_str: std::string::String = format!("{}",conf.server_address);
        // let server_addr = server_str.as_str();
        let server_addr = config["server_address"].to_string().replace("\"", "");
        let channel = Channel::from_shared(server_addr)
            .unwrap()
            .tcp_keepalive(Some(Duration::from_secs(10)))
            .timeout(Duration::from_secs(
                config["channel_timeout_seconds"].as_u64().unwrap(),
            ))
            .tls_config(tls)
            .unwrap()
            .connect_timeout(Duration::from_secs(30))
            .connect_lazy();
        // Step 1: Wrap the base channel with the interceptor
        let interceptor = AuthInterceptor {
            token: format!("Bearer {}", config["token"].to_string().replace("\"", "")),
            secret_key: config["secret_key"].to_string().replace("\"", ""),
            secret_value: config["secret_value"].to_string().replace("\"", ""),
        };
        let intercepted_channel = InterceptedService::new(channel, interceptor);

        // Client with compression properties
        let mut client = DataTransmissionClient::new(intercepted_channel)
            .send_compressed(tonic::codec::CompressionEncoding::Gzip)
            .accept_compressed(tonic::codec::CompressionEncoding::Gzip);
        println!("gRPC CLient with TLS & Security");
        // Unary call
        let request = Request::new(Data {
            request_msg: "UnaryUser".into(),
        });
        let response = client.unary_server(request).await?;
        println!("Unary Response: {:?}", response.into_inner().message);

        // Server streaming
        let request = Request::new(Data {
            request_msg: "StreamUser".into(),
        });
        let mut stream = client.server_streaming(request).await?.into_inner();
        println!("Server Streaming:");
        while let Some(reply) = stream.message().await? {
            println!("  -> {}", reply.request_msg);
        }

        // Client streaming
        let in_stream = echo_requests_iter().take(4);
        let response = client.client_streaming(in_stream).await.unwrap();
        println!(
            "Client Streaming Response: {}",
            response.into_inner().request_msg
        );

        // Bidirectional streaming
        println!("Chat Streaming Response");
        let in_stream = echo_requests_iter().take(4);
        let response = client.chat_streaming(in_stream).await.unwrap();
        let mut resp_stream = response.into_inner();

        while let Some(received) = resp_stream.next().await {
            let received = received.unwrap();
            println!("\treceived message: `{}`", received.request_msg);
        }
    } else {
        let server_addr = config["server_address"].to_string().replace("\"", "");

        let mut client = DataTransmissionClient::connect(server_addr).await?;

        println!("gRPC CLient without TLS & Security");


        // Unary call
        let request = Request::new(Data {
            request_msg: "UnaryUser".into(),
        });
        let response = client.unary_server(request).await?;
        println!("Unary Response: {:?}", response.get_ref().message);

        // Server streaming
        let request = Request::new(Data {
            request_msg: "StreamUser".into(),
        });
        let mut stream = client.server_streaming(request).await?.into_inner();
        println!("Server Streaming:");
        while let Some(reply) = stream.message().await? {
            println!("  -> {}", reply.request_msg);
        }

        // Client streaming
        let in_stream = echo_requests_iter().take(4);
        let response = client.client_streaming(in_stream).await.unwrap();
        println!(
            "Client Streaming Response: {}",
            response.into_inner().request_msg
        );

        // Bidirectional streaming
        println!("Chat Streaming Response");
        let in_stream = echo_requests_iter().take(4);
        let response = client.chat_streaming(in_stream).await.unwrap();
        let mut resp_stream = response.into_inner();

        while let Some(received) = resp_stream.next().await {
            let received = received.unwrap();
            println!("\treceived message: `{}`", received.request_msg);
        }
    };

    Ok(())
}
