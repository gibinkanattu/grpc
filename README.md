# gRPC
[gRPC](https://grpc.io/docs/what-is-grpc/introduction) server and client implemented with all four gRPC call Types:
* Unary
* Server Streaming
* Client Streaming
* Bi-Directional Streaming

## Security features included
* SSL/TLS certificate encryption
* JWT Token based authentication
* Secret key-value pair passed on each requests

### How to Execute

#### Development
* Server
> cargo run --bin server
* Client
> cargo build --bin client
#### Production
Will build server and client with release properties
> cargo build --release