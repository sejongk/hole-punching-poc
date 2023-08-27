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
            Arg::with_name("stun")
                .required_unless("FULLHELP")
                .takes_value(true)
                .default_value("stun.sipnet.net:3478")
                .long("stun")
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

    let stun = matches.value_of("stun").unwrap();
    let allowed = matches.value_of("allowed").unwrap();

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

    // allowed
    println!("send to allowed");
    let socket = TcpSocket::new_v4()?;
    socket.set_reuseaddr(true)?;
    socket.bind("0.0.0.0:8080".parse().unwrap())?;

    if let Ok(_) = socket.connect(allowed.parse().unwrap()).await {
        // Successfully connected, continue with the connection
    } else {
        // Ignore the "Network is unreachable" error
        // Handle other errors if needed
        println!("Connection failed, but ignoring the error.");
    }
    stream.shutdown().await?;

    // open server
    println!("open server");
    let socket = TcpSocket::new_v4()?;
    socket.set_reuseaddr(true)?;
    socket.bind("0.0.0.0:8080".parse().unwrap())?;

    let listener = socket.listen(1024)?;
    println!("Listening on: {}", listener.local_addr()?);

    while let Ok((mut tcp_stream, _)) = listener.accept().await {
        let mut buffer: [u8; 1024] = [0; 1024];
        let _size = tcp_stream.read(&mut buffer).await;
        tcp_stream.write_all(b"Hello, client").await?;
        tcp_stream.flush().await?;
    }

    Ok(())
}
