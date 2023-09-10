use std::sync::Arc;

use clap::{App, Arg};

use stun::message::*;
use stun::xoraddr::*;
use stun::Error;
use tokio::net::UdpSocket;

use std::net::SocketAddr;
use std::time::Duration;

mod udp_server;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let app = App::new("STUN PoC")
        .arg(
            Arg::with_name("stun_cands_file")
                .takes_value(true)
                .long("stun_cands_file"),
        )
        .arg(
            Arg::with_name("peer_addr")
                .takes_value(true)
                .long("peer_addr"),
        );

    let matches: clap::ArgMatches = app.clone().get_matches();

    let stun_cands_file = matches.value_of("stun_cands_file").unwrap().to_string();
    let peer_addr = matches.value_of("peer_addr").unwrap().to_string();
    let bind_port = 8080;
    let request_period = Duration::from_secs(10);

    let bind_addr = SocketAddr::from(([0, 0, 0, 0], bind_port));
    let sock = UdpSocket::bind(bind_addr).await?;
    let conn = Arc::new(sock);

    tokio::spawn(udp_server::keep_connecting_to_available_stun(
        conn.clone(),
        stun_cands_file,
        request_period,
    ));

    let mut msg = Message::new();
    let mut buf = [0; 1024];

    loop {
        println!("Send a request to the peer: {}", peer_addr);
        conn.send_to(b"DMS:CONSENSUS:PING", peer_addr.clone())
            .await?;

        match conn.recv_from(&mut buf).await {
            Ok((len, addr)) => match msg.unmarshal_binary(&buf) {
                Ok(_) => {
                    println!("{:?} bytes received from the STUN ({:?})", len, addr);
                    let mut xor_addr = XorMappedAddress::default();
                    xor_addr.get_from(&msg).unwrap();
                    println!("Got STUN response: {xor_addr}");
                }
                Err(_) => {
                    println!("{:?} bytes received from the Peer ({:?})", len, addr);
                    let len = conn.send_to(&buf[..len], addr).await?;
                    println!("{:?} bytes sent", len);
                    break;
                }
            },
            Err(_) => {
                println!("error!");
            }
        }
    }

    Ok(())
}
