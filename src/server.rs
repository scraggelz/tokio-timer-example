#![feature(async_closure)]

use std::{
    collections::HashMap,
    env,
    io::Error as IoError,
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use std::any::type_name;
use futures_channel::mpsc::{unbounded, UnboundedSender};
use futures_util::{future, pin_mut, stream::TryStreamExt, StreamExt};

use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout;
use tokio::sync::oneshot;

use std::time::Duration;
use tungstenite::protocol::Message;

use crossbeam_channel;

type Tx = UnboundedSender<Message>;
type PeerMap = Arc<Mutex<HashMap<SocketAddr, Tx>>>;

fn type_of<T>(_: T) -> &'static str {
    type_name::<T>()
}

fn remove_whitespace(s: &str) -> String {
    s.split_whitespace().collect()
}

async fn handle_connection(peer_map: PeerMap, raw_stream: TcpStream, addr: SocketAddr) {
    println!("Incoming TCP connection from: {}", addr);

    let ws_stream = tokio_tungstenite::accept_async(raw_stream)
        .await
        .expect("Error during the websocket handshake occurred");
    println!("WebSocket connection established: {}", addr);

    // Insert the write part of this peer to the peer map.
    let (tx, rx) = unbounded();
    peer_map.lock().unwrap().insert(addr, tx);

    let (outgoing, incoming) = ws_stream.split();

    let mut recieved_start = false;
    let mut recieved_stop  = false;
    let (stop_start_write, mut stop_start_read) = crossbeam_channel::unbounded();

    let broadcast_incoming = incoming.try_for_each(|msg| {
        println!("Received a message from {}: {}", addr, msg.to_text().unwrap());
        let data = remove_whitespace(msg.to_text().unwrap());
        // println!("{}", type_of(data.clone()));
        if data == "start" {
            recieved_start = true;
            let mut stop_start_read_inner = stop_start_read.clone();
            tokio::spawn( async move  {
                if let Ok(msg) = stop_start_read_inner.recv_timeout(Duration::from_secs(5)) {
                    println!("Received stop");
                }
                else {
                    println!("Timeout!");
                }
            });
        }

        if data == "stop" && recieved_start == true {
            stop_start_write.send("stop");
            recieved_start = false;
        }
        else {
            println!("doesn't match shit.")
        }
        let peers = peer_map.lock().unwrap();

        // We want to broadcast the message to everyone except ourselves.
        let broadcast_recipients =
            peers.iter().filter(|(peer_addr, _)| peer_addr == &&addr).map(|(_, ws_sink)| ws_sink);

        for recp in broadcast_recipients {
            recp.unbounded_send(msg.clone()).unwrap();
        }

        future::ok(())
    });



    let receive_from_others = rx.map(Ok).forward(outgoing);

    pin_mut!(broadcast_incoming, receive_from_others);
    future::select(broadcast_incoming, receive_from_others).await;

    println!("{} disconnected", &addr);
    peer_map.lock().unwrap().remove(&addr);
}

#[tokio::main]
async fn main() -> Result<(), IoError> {
    let addr = env::args().nth(1).unwrap_or_else(|| "127.0.0.1:8080".to_string());

    let state = PeerMap::new(Mutex::new(HashMap::new()));

    // Create the event loop and TCP listener we'll accept connections on.
    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");
    println!("Listening on: {}", addr);

    // Let's spawn the handling of each connection in a separate task.
    while let Ok((stream, addr)) = listener.accept().await {
        tokio::spawn(handle_connection(state.clone(), stream, addr));
    }

    Ok(())
}
