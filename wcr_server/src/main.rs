use std::sync::mpsc;
use std::io::ErrorKind;
use std::io::prelude::*;
use std::thread;
use std::env;
use std::net::{TcpListener,TcpStream,SocketAddr,Shutdown};
use std::time::Duration;
use magic_crypt::{MagicCrypt256,MagicCryptTrait,new_magic_crypt};

const BANNER: &[u8] = b"\n\n==================================================\n       Bem vindo ao Wasteland Chat Reborn !       \n==================================================\n\n";
const READ_DELAY_MS: u64 = 10;

//This funcion mesures the data, encrypts it, and make the first byte the size (max 200)
//In short, it makes the necessary package format for this server to use
fn packet_maker(bytes: &[u8],mc: MagicCrypt256) -> Vec<u8> {
    let mut data = mc.encrypt_bytes_to_bytes(bytes);
    data.insert(0, data.len() as u8);
    data
}

fn handle_client(mut socket: TcpStream, addr: SocketAddr,tx: mpsc::Sender<Vec<u8>>, mc: MagicCrypt256) {
    socket.write(&packet_maker(BANNER,mc.clone())).unwrap();
    let mut sus_spy: bool = true;
    loop {
        let mut buf = [0u8;255];
        match socket.read(&mut buf) {
            Ok(_) => {
                if &buf[0] < &1 {
                    break;
                }
                let msg_size = buf[0] as usize;
                if sus_spy {
                    println!("[SPYBUSTER]=> Processed buf: {:?}",&buf[1..msg_size+1]);
                    println!("[SPYBUSTER]=> Spy check...");
                    if let Err(_) = mc.decrypt_bytes_to_bytes(&buf[1..msg_size+1]) {
                        println!("[SPYBUSTER]=> Failed !");
                        println!("[SPYBUSTER]=> Kicking spy...");
                        socket.write(b"\n\n\nFUCK YOU !\n\n\n").unwrap();
                        break;
                    }
                    println!("[SPYBUSTER]=> Passed !");
                    sus_spy = false;
                }
                println!("[INFO]: {} => msg_len: {}",addr,buf[0]);
                tx.send(buf.to_vec()).expect("Failed tx send !");
            },
            Err(ref err) if err.kind() == ErrorKind::WouldBlock => (),
            Err(_) => {
                break;
            }
        }
        thread::sleep(Duration::from_millis(READ_DELAY_MS))
    }
    println!("[INFO]: We lost em !: {}",addr);
    socket.shutdown(Shutdown::Both).unwrap();
    tx.send(packet_maker(format!("<SERVER>: [{}] desconectado !, sus_spy: {}",addr,sus_spy).as_bytes(),mc.clone())).unwrap();
}

fn main() {
    let port: String = env::args().nth(1).unwrap();
    let pass: String = env::args().nth(2).unwrap();

    println!("[INFO]: Starting encryption...");

    let mut client_count: usize = 0;
    let mc = new_magic_crypt!(pass,256);
    
    println!("[INFO]: Starting server on port: {}...",port);

    let mut clients: Vec<TcpStream> = Vec::new();
    let (tx,rx) = mpsc::channel::<Vec<u8>>();
    let stream = TcpListener::bind(format!("0.0.0.0:{}",port)).expect("Failed to bind server");
    stream.set_nonblocking(true).expect("Failed to initialize non-blocking");

    println!("[INFO]: Server started !");
    println!("[INFO]: Waiting for connections...");

    loop {
        if let Ok((socket, addr)) = stream.accept() {
            println!("[INFO]: Conection: {}",addr);

            let tx = tx.clone();
            let mc = mc.clone();
            clients.push(socket.try_clone().expect("Socket clone failed !"));
            thread::spawn(move||handle_client(socket,addr,tx,mc));
        }

        //Message forwarder
        if let Ok(msg) = rx.try_recv() {
            clients = clients.into_iter().filter_map(|mut client|{
                client.write_all(&msg).map(|_|client).ok()
            }).collect::<Vec<TcpStream>>();

            if clients.len() != client_count {
                client_count = clients.len();
                tx.send(packet_maker(format!("<SERVER>: {} clientes conectados.",client_count).as_bytes(),mc.clone())).unwrap();
            }
            println!("[INFO]: Clients connected: {}",clients.len())
        }

        thread::sleep(Duration::from_millis(10))
    }

}