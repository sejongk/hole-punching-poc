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
use tokio::io::Interest;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let app = App::new("STUN Client")
        .version("0.1.0")
        .author("Rain Liu <yliu@webrtc.rs>")
        .about("An example of STUN Client")
        .arg(
            Arg::with_name("FULLHELP")
                .help("Prints more detailed help information")
                .long("fullhelp"),
        )
        .arg(
            Arg::with_name("stun")
                .required_unless("FULLHELP")
                .takes_value(true)
                .default_value("stun.sipnet.net:3478")
                .long("stun")
                .help("STUN Server"),
        )
        .arg(
            Arg::with_name("server")
                .required_unless("FULLHELP")
                .takes_value(true)
                .long("server")
                .help("TCP server"),
        );

    let matches = app.clone().get_matches();

    let stun = matches.value_of("stun").unwrap();
    let server = matches.value_of("server").unwrap();

    // stun
    let socket = TcpSocket::new_v4()?;
    socket.set_reuseaddr(true)?;
    socket.bind("0.0.0.0:8080".parse().unwrap())?;

    println!("Local address: {}", socket.local_addr()?);

    let stun_addrs: Vec<SocketAddr> = stun.to_socket_addrs().unwrap().collect();
    println!("{}", stun_addrs[0]);
    let mut stream = socket.connect(stun_addrs[0]).await?;
    let mut msg = Message::new();
    msg.build(&[Box::<TransactionId>::default(), Box::new(BINDING_REQUEST)])?;
    stream.write_all(&msg.raw).await?;

    loop {
        let ready = stream
            .ready(Interest::READABLE | Interest::WRITABLE)
            .await?;

        if ready.is_readable() {
            let mut data: Vec<u8> = vec![0; 1024];
            match stream.try_read(&mut data) {
                Ok(n) => {
                    println!("read {} bytes", n);
                    match msg.unmarshal_binary(&data) {
                        Ok(_) => {
                            let mut xor_addr = XorMappedAddress::default();
                            xor_addr.get_from(&msg)?;
                            println!("Got response: {xor_addr}");
                        }
                        Err(_) => {
                            println!("error!");
                        }
                    }
                }
                Err(e) => {
                    return Err(e.into());
                }
            }
            break;
        }
    }

    // conect to server
    let socket = TcpSocket::new_v4()?;
    socket.set_reuseaddr(true)?;
    socket.bind("0.0.0.0:8080".parse().unwrap())?;
    if let Some(mut tcp_stream) = socket.connect(server.parse().unwrap()).await.ok() {
        tcp_stream.write_all(b"Hello server").await?;
        tcp_stream.flush().await?;
        let mut buffer = [0; 1024];
        let _ = tcp_stream.read(&mut buffer).await?;
        let message = String::from_utf8_lossy(&buffer);
        println!("Server says: {}", message);
    }

    Ok(())
}
