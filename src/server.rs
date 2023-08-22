use std::sync::Arc;

use clap::{App, Arg};
use stun::agent::*;
use stun::client::*;
use stun::message::*;
use stun::xoraddr::*;
use stun::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpSocket, UdpSocket};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let mut app = App::new("STUN Client")
        .version("0.1.0")
        .author("Rain Liu <yliu@webrtc.rs>")
        .about("An example of STUN Client")
        .arg(
            Arg::with_name("FULLHELP")
                .help("Prints more detailed help information")
                .long("fullhelp"),
        )
        .arg(
            Arg::with_name("server")
                .required_unless("FULLHELP")
                .takes_value(true)
                .default_value("stun.l.google.com:19302")
                .long("server")
                .help("STUN Server"),
        )
        .arg(
            Arg::with_name("allowed")
                .required_unless("FULLHELP")
                .takes_value(true)
                .long("allowed")
                .help("STUN allowed client"),
        );

    let matches: clap::ArgMatches = app.clone().get_matches();

    if matches.is_present("FULLHELP") {
        app.print_long_help().unwrap();
        std::process::exit(0);
    }

    let server = matches.value_of("server").unwrap();

    // let conn = UdpSocket::bind("0.0.0.0:8080").await?;

    // println!("allowed: {allowed}");
    // let buf: [u8; 1024] = [0; 1024];
    // let _ = conn.send_to(&buf, server);

    let sock = UdpSocket::bind("0.0.0.0:8080").await?;
    println!("Local address: {}", sock.local_addr()?);

    let allowed = matches.value_of("allowed").unwrap();
    let len = sock.send_to(b"hello world", allowed).await?;

    println!("Sent {} bytes", len);

    let (handler_tx, mut handler_rx) = tokio::sync::mpsc::unbounded_channel();

    println!("Connecting to: {server}");
    let arc_conn = Arc::new(sock);

    arc_conn.connect(server).await?;
    let mut client = ClientBuilder::new().with_conn(arc_conn).build()?;

    let mut msg = Message::new();
    msg.build(&[Box::<TransactionId>::default(), Box::new(BINDING_REQUEST)])?;

    client.send(&msg, Some(Arc::new(handler_tx))).await?;

    if let Some(event) = handler_rx.recv().await {
        let msg = event.event_body?;
        let mut xor_addr = XorMappedAddress::default();
        xor_addr.get_from(&msg)?;
        println!("Got response: {xor_addr}");
    }

    client.close().await?;

    // open server
    let tcp_socket = TcpSocket::new_v4()?;
    tcp_socket.bind("0.0.0.0:8080".parse().unwrap())?;
    tcp_socket.set_reuseaddr(true)?;
    let listener = tcp_socket.listen(1024).unwrap();
    println!("Listening on: {}", listener.local_addr()?);

    while let Ok((mut tcp_stream, _)) = listener.accept().await {
        let mut buffer = [0; 1024];
        let _size = tcp_stream.read(&mut buffer).await;
        tcp_stream.write_all(b"Hello, client").await?;
        tcp_stream.flush().await?;
    }
    Ok(())
}
