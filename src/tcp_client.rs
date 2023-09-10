use std::sync::Arc;

use clap::{App, Arg};
use std::net::{SocketAddr, ToSocketAddrs};
use stun::addr::*;
use stun::agent::*;
use stun::client::*;
use stun::message::*;
use stun::xoraddr::*;
use stun::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpSocket, TcpStream, UdpSocket};

use std::io;
use std::time::Duration;
use tokio::io::Interest;

use hyper::client::HttpConnector;
use hyper::http::Uri;
use hyper::Client;
use std::net::IpAddr;

use futures::future::BoxFuture;
use hyper::http::{Request, Response, StatusCode};
use hyper::service::Service;
use std::task::{self, Poll};

mod tcp_server;

#[derive(Clone)]
struct MyConnector {
    port: u16,
}

impl Service<Uri> for MyConnector {
    type Response = TcpStream;
    type Error = std::io::Error;
    type Future = BoxFuture<'static, Result<TcpStream, Self::Error>>;

    fn poll_ready(&mut self, _: &mut task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, uri: Uri) -> Self::Future {
        let bind_port = self.port;
        Box::pin(async move {
            let addr = SocketAddr::from(([0, 0, 0, 0], bind_port));
            let socket = TcpSocket::new_v4().unwrap();
            socket.set_reuseaddr(true).unwrap();
            socket.bind(addr).unwrap();

            let host = uri.host().unwrap();
            let port = uri.port_u16().unwrap_or(80);

            let addr_str: String = format!("{}:{}", host, port);
            let dest_addr = addr_str.to_socket_addrs()?.next().unwrap();
            let result = socket.connect(dest_addr).await;

            result
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // let app = App::new("STUN Client")
    //     .version("0.1.0")
    //     .author("Rain Liu <yliu@webrtc.rs>")
    //     .about("An example of STUN Client")
    //     .arg(
    //         Arg::with_name("FULLHELP")
    //             .help("Prints more detailed help information")
    //             .long("fullhelp"),
    //     )
    //     .arg(
    //         Arg::with_name("stun")
    //             .required_unless("FULLHELP")
    //             .takes_value(true)
    //             .default_value("stun.sipnet.net:3478")
    //             .long("stun")
    //             .help("STUN Server"),
    //     )
    //     .arg(
    //         Arg::with_name("server")
    //             .required_unless("FULLHELP")
    //             .takes_value(true)
    //             .long("server")
    //             .help("TCP server"),
    //     );

    // let matches = app.clone().get_matches();

    // let stun = matches.value_of("stun").unwrap();
    // let server = matches.value_of("server").unwrap();

    let mut stun_cands: Vec<&str> = Vec::new();
    stun_cands.push("stun.sipnet.net:3478");
    let bind_port = 8080;
    let request_period = Duration::from_secs(10);

    tokio::spawn(server::keep_connecting_to_available_stun(
        stun_cands,
        bind_port,
        request_period,
    ));

    let local_addr = "127.0.0.1".parse::<IpAddr>().unwrap();
    let mut connector = HttpConnector::new();
    connector.set_reuse_address(true);
    connector.set_local_address(Some(local_addr));

    let client = Client::builder().build::<_, hyper::Body>(MyConnector { port: bind_port });
    let url = "http://www.example.com".parse::<Uri>().unwrap();
    let response = client.get(url).await.unwrap();

    let body_bytes = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();

    println!("Response: {}", body_str);

    Ok(())
}
