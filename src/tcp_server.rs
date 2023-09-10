use axum::{routing::get, Router};
use std::net::{SocketAddr, TcpListener, ToSocketAddrs};
use stun::agent::*;
use stun::message::*;
use stun::xoraddr::*;
use stun::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpSocket;
use tokio::time;
use tokio::time::timeout;

use nix::sys::socket::setsockopt;
use std::os::unix::io::AsRawFd;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let bind_port = 8080;
    let request_period = Duration::from_secs(10);

    let mut stun_cands: Vec<&str> = Vec::new();
    stun_cands.push("stun.sipnet.net:3478");
    let mut allowed_clients: Vec<&str> = Vec::new();
    allowed_clients.push("stun.sipnet.net:3478");

    tokio::spawn(keep_connecting_to_available_stun(
        stun_cands,
        bind_port,
        request_period,
    ));

    tokio::spawn(keep_connecting_to_allowed_clients(
        allowed_clients,
        bind_port,
        request_period,
    ));

    // open server
    println!("Open http server");
    let app = Router::new().route("/", get(|| async { "Hello, world!" }));

    let addr = SocketAddr::from(([0, 0, 0, 0], bind_port));
    let listener = TcpListener::bind(addr).unwrap();

    // Get the file descriptor of the socket
    let fd = listener.as_raw_fd();
    // Enable SO_REUSEADDR option
    setsockopt(fd, nix::sys::socket::sockopt::ReuseAddr, &true).expect("Failed to setsockopt");

    println!("listening on {}", addr);
    axum::Server::from_tcp(listener)
        .unwrap()
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}

pub async fn keep_connecting_to_available_stun(
    stun_cands: Vec<&str>,
    bind_port: u16,
    request_period: Duration,
) {
    loop {
        for stun_cand in stun_cands.clone() {
            let addr = SocketAddr::from(([0, 0, 0, 0], bind_port));
            let socket = TcpSocket::new_v4().unwrap();
            socket.set_reuseaddr(true).unwrap();
            socket.bind(addr).unwrap();

            let stun_ipv4s: Vec<SocketAddr> = stun_cand
                .to_socket_addrs()
                .unwrap()
                .filter(|addr| addr.is_ipv4())
                .collect();

            match socket.connect(stun_ipv4s[0]).await {
                Ok(mut stream) => {
                    println!("Connected to STUN {}", stun_ipv4s[0]);
                    // Send a STUN request message.
                    let mut msg: Message = Message::new();
                    msg.build(&[Box::<TransactionId>::default(), Box::new(BINDING_REQUEST)])
                        .unwrap();
                    stream.write_all(&msg.raw).await.unwrap();

                    // Receive a STUN response.
                    let mut response = vec![0; 1024];
                    match stream.read(&mut response).await {
                        Ok(_) => match msg.unmarshal_binary(&response) {
                            Ok(_) => {
                                let mut xor_addr = XorMappedAddress::default();
                                xor_addr.get_from(&msg).unwrap();
                                println!("Got STUN response: {xor_addr}");

                                // Break when the process succeeds.
                                drop(stream);
                                break;
                            }
                            Err(_) => {
                                println!("error!");
                            }
                        },
                        Err(_) => {
                            println!("error!");
                        }
                    }
                }
                Err(err) => {
                    eprintln!("Error connecting to STUN {}: {}", stun_ipv4s[0], err);
                }
            }
        }

        time::sleep(request_period).await;
    }
}

async fn keep_connecting_to_allowed_clients(
    clients: Vec<&str>,
    bind_port: u16,
    request_period: Duration,
) {
    loop {
        for client in clients.clone() {
            let addr = SocketAddr::from(([0, 0, 0, 0], bind_port));
            let socket = TcpSocket::new_v4().unwrap();
            socket.set_reuseaddr(true).unwrap();
            socket.bind(addr).unwrap();

            let client_ipv4s: Vec<SocketAddr> = client
                .to_socket_addrs()
                .unwrap()
                .filter(|addr| addr.is_ipv4())
                .collect();

            match timeout(Duration::from_millis(1), socket.connect(client_ipv4s[0])).await {
                Ok(Ok(stream)) => drop(stream),
                Ok(Err(_)) => {}
                Err(_) => {}
            }
        }

        time::sleep(request_period).await;
    }
}
