use std::sync::Arc;

use clap::{App, Arg};
use stun::agent::*;
use stun::message::*;
use stun::xoraddr::*;
use stun::Error;
use tokio::net::UdpSocket;

use tokio::time;

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::net::SocketAddr;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let app = App::new("STUN PoC")
        .arg(
            Arg::with_name("stun_cands_file")
                .takes_value(true)
                .long("stun_cands_file"),
        )
        .arg(
            Arg::with_name("allowed_clients_file")
                .takes_value(true)
                .long("allowed_clients_file"),
        );

    let matches: clap::ArgMatches = app.clone().get_matches();

    let stun_cands_file = matches.value_of("stun_cands_file").unwrap().to_string();
    let allowed_clients_file = matches
        .value_of("allowed_clients_file")
        .unwrap()
        .to_string();
    let bind_port = 8080;
    let request_period = Duration::from_secs(10);

    let bind_addr = SocketAddr::from(([0, 0, 0, 0], bind_port));
    let sock = UdpSocket::bind(bind_addr).await?;
    let conn = Arc::new(sock);

    tokio::spawn(keep_connecting_to_available_stun(
        conn.clone(),
        stun_cands_file,
        request_period,
    ));

    tokio::spawn(keep_connecting_to_allowed_clients(
        conn.clone(),
        allowed_clients_file,
        request_period,
    ));

    // open server
    println!("Listening on: {}", conn.clone().local_addr()?);
    let mut msg = Message::new();
    let mut buf = [0; 1024];
    loop {
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
                }
            },
            Err(_) => {
                println!("error!");
            }
        }
    }
    Ok(())
}

pub async fn keep_connecting_to_available_stun(
    conn: Arc<UdpSocket>,
    stun_cands_file_name: String,
    request_period: Duration,
) -> Result<(), Error> {
    loop {
        let file = File::open(stun_cands_file_name.clone())?;
        let reader = BufReader::new(file);

        // TODO(sejongk): try to send request without the below msg payload
        let mut msg = Message::new();
        msg.build(&[Box::<TransactionId>::default(), Box::new(BINDING_REQUEST)])?;

        for line in reader.lines() {
            match line {
                Ok(addr) => {
                    conn.send_to(&msg.raw, addr).await?;
                }
                Err(err) => {
                    eprintln!("Error reading line: {}", err);
                }
            }
        }
        time::sleep(request_period).await;
    }

    Ok(())
}

async fn keep_connecting_to_allowed_clients(
    conn: Arc<UdpSocket>,
    allowed_clients_file_name: String,
    request_period: Duration,
) -> Result<(), Error> {
    loop {
        let file = File::open(allowed_clients_file_name.clone())?;
        let reader = BufReader::new(file);

        for line in reader.lines() {
            match line {
                Ok(addr) => {
                    conn.send_to(b"DMS:NAT_TRAVERSAL:PING", addr).await?;
                }
                Err(err) => {
                    eprintln!("Error reading line: {}", err);
                }
            }
        }

        time::sleep(request_period).await;
    }
}
