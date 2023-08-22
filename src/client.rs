use std::sync::Arc;

use clap::{App, Arg};
use stun::agent::*;
use stun::client::*;
use stun::message::*;
use stun::xoraddr::*;
use stun::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpSocket;
use tokio::net::UdpSocket;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let app = App::new("STUN Client")
        .version("0.1.0")
        .author("Rain Liu <yliu@webrtc.rs>")
        .about("An example of STUN Client")
        .arg(
            Arg::with_name("stun")
                .takes_value(true)
                .default_value("stun.l.google.com:19302")
                .long("stun")
                .help("STUN Server"),
        )
        .arg(
            Arg::with_name("server")
                .takes_value(true)
                .long("server")
                .help("Server"),
        );

    let matches = app.clone().get_matches();

    let server = matches.value_of("server").unwrap();
    let stun = matches.value_of("stun").unwrap();

    // stun
    let (handler_tx, mut handler_rx) = tokio::sync::mpsc::unbounded_channel();

    let conn = UdpSocket::bind("0.0.0.0:8080").await?;
    let arc_conn = Arc::new(conn);
    let arc_conn2 = arc_conn.clone();
    println!("Local address: {}", arc_conn.local_addr()?);

    println!("server ip: {}", server);
    let len = arc_conn2.send_to(b"hello world", server).await?;
    println!("{:?} bytes sent", len);

    println!("Connecting to: {stun}");
    arc_conn.connect(stun).await?;

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

    // conect to server
    // let socket = TcpSocket::new_v4()?;
    // socket.bind("0.0.0.0:8080".parse().unwrap())?;
    // socket.set_reuseaddr(true)?;
    // if let Some(mut tcp_stream) = socket.connect(server.parse().unwrap()).await.ok() {
    //     tcp_stream.write_all(b"Hello server").await?;
    //     tcp_stream.flush().await?;
    //     let mut buffer = [0; 1024];
    //     let _ = tcp_stream.read(&mut buffer).await?;
    //     let message = String::from_utf8_lossy(&buffer);
    //     println!("Server says: {}", message);
    // }

    Ok(())
}
