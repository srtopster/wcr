use std::net::{TcpStream,Shutdown};
use std::thread;
use std::time::Duration;
use std::env;
use std::io::prelude::*;
use std::io::{stdin,stdout,ErrorKind};
use rustyline_async::{Readline,SharedWriter,ReadlineError};
use magic_crypt::{MagicCrypt256,MagicCryptTrait,new_magic_crypt};
use crossterm::{ExecutableCommand,cursor};
use crossterm::terminal::{SetTitle,Clear,ClearType};
use tokio;

const READ_DELAY_MS: u64 = 10;
const MAX_NICK_LEN: usize = 16;

fn packet_maker(bytes: &[u8],mc: MagicCrypt256) -> Vec<u8> {
    let mut data = mc.encrypt_bytes_to_bytes(bytes);
    data.insert(0, data.len() as u8);
    data
}

fn recv_thread(mut socket: TcpStream,mut stdout: SharedWriter,mc: MagicCrypt256) {
    //manda uma mensagem de check para ver se a encryptação ta certa
    loop {
        let mut buf = [0u8;255];
        match socket.read(&mut buf) {
            Ok(_) => {
                //println!("{:?}",buf);
                let msg_size = buf[0] as usize;
                if let Ok(decrypted_msg) = mc.decrypt_bytes_to_bytes(&buf[1..msg_size+1]) {
                    let utf8_msg = String::from_utf8(decrypted_msg).unwrap();
                    if utf8_msg.len() < 1 {
                        break;
                    }
                    writeln!(stdout,"{}",utf8_msg).unwrap();
                } else {
                    writeln!(stdout,"Falha na decriptação !\nParando...").unwrap();
                    break;
                }
            },
            Err(ref err) if err.kind() == ErrorKind::WouldBlock => (),
            Err(_) => {
                break;
            }
        }
        thread::sleep(Duration::from_millis(READ_DELAY_MS));
    }
    writeln!(stdout,"Conexão perdida !").unwrap();
}

fn read_stdin() -> String {
    stdout().flush().unwrap();
    let mut buf = String::new();
    stdin().read_line(&mut buf).unwrap();
    buf.trim().to_owned()
}

#[tokio::main]
async fn main() {
    stdout()
        .execute(SetTitle("W.C.R Client")).unwrap()
        .execute(Clear(ClearType::All)).unwrap()
        .execute(cursor::MoveTo(0,0)).unwrap();

    let args: Vec<String> = env::args().collect();
    let ip_port: String = if args.len() < 2 {
        print!("Digite o ip:porta do server: ");
        read_stdin()
    } else {
        args.get(1).unwrap().to_owned()
    };
    print!("Digite o seu apelido (maximo {} characteres): ",MAX_NICK_LEN);
    let nick_name = read_stdin();
    if nick_name.len() > MAX_NICK_LEN {
        println!("Apelido muito grande !");
        return;
    }
    print!("Digite a senha para decriptar o servidor: ");
    let pass = read_stdin();
    let mc = new_magic_crypt!(pass,256);

    stdout()
        .execute(Clear(ClearType::All)).unwrap()
        .execute(cursor::MoveTo(0,0)).unwrap();

    let (mut rl,mut stdout) = Readline::new("> ".to_owned()).unwrap();
    rl.should_print_line_on(false, true);
    
    println!("Tentando se conectar...");
    if let Ok(mut socket) = TcpStream::connect(ip_port) {
        socket.set_nonblocking(true).unwrap();
        let socket_clone = socket.try_clone().unwrap();
        let stdout_clone = stdout.clone();
        let mc_clone = mc.clone();
        println!("Conectado !");
        let t = thread::spawn(move||recv_thread(socket_clone,stdout_clone,mc_clone));

        //Terminal IO loop
        loop {
            match rl.readline().await {
                Ok(line) => {
                    if line.as_bytes().len() < 200  {
                        if line.len() < 1 {
                            continue;
                        }
                        let data = packet_maker(format!("[{}]: {}",nick_name,line).as_bytes(), mc.clone());
                        if let Err(_) = socket.write(&data) {
                            writeln!(stdout,"Falha ao enviar mensagem !").unwrap();
                            break;
                        }
                    } else { 
                        writeln!(stdout,"Mensagem muito grande, não vou arrumar, maximo de 200 bytes mesmo, e foda-se, fui eu que fiz e eu quero assim.").unwrap();
                    }
                }
                Err(ReadlineError::Closed)|Err(ReadlineError::Eof)|Err(ReadlineError::Interrupted) => {
                    writeln!(stdout,"Desconectando...").unwrap();
                    break;
                },
                Err(err) => {
                    writeln!(stdout,"Erro fatal: {}",err).unwrap();
                    break;
                }
            }
        }
        writeln!(stdout,"Finalizando conexão...").unwrap();
        socket.shutdown(Shutdown::Both).unwrap();

        writeln!(stdout,"Esperando thread morrer...").unwrap();
        t.join().unwrap();
    } else {
        println!("Conexão falhou !")
    };
    writeln!(stdout,"Finalizando terminal...").unwrap();
    rl.flush().unwrap();
    drop(rl);
    println!("Aperte enter para sair...");
    stdin().read(&mut [0u8;0]).unwrap();
}