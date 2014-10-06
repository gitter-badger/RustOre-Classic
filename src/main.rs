extern crate time;
extern crate curl;
extern crate flate2;

use curl::http;

use std::io::{Listener, Acceptor, IoResult, IoError, InvalidInput};
use std::io::net::tcp::{TcpListener, TcpStream};

use std::io::timer;
use std::time::Duration;

use std::io::MemWriter;
use flate2::writer::GzEncoder;

use config::Configuration;
use packets::{MCPackets};


mod packets;
mod config;

struct Packet{
	packet_id: u8,
	packet_len: uint,
	data: Vec<u8>
}

fn send_heartbeat(config: Configuration) -> IoResult<()>{
	let start_time: f64 = time::precise_time_s();
    let response = http::handle().get(format!("https://minecraft.net/heartbeat.jsp?port={:u}&max={:u}&name={:s}&public={:s}&version=7&salt={:s}&users=0", config.port, config.max_players, config.server_name.as_slice(), config.is_public.as_slice(), config.salt.as_slice())).exec().unwrap();
    println!("Heartbeat done! Took {} seconds.", time::precise_time_s() - start_time);
    Ok(())
}

fn handle_connection(config: Configuration, mut conn: TcpStream) -> IoResult<()>{
	let ip = try!(conn.peer_name()).ip;
	println!("{} is connecting to us...", ip);
	loop{
		let packet = parse_packet(config.clone(), conn.clone());
		println!("{}", packet.packet_id);
		
		if packet.packet_id == 0x00{
			conn.send_server_ident(config.clone());
			
			//Send debug level data
			conn.send_level_init();
			let mut data: Vec<u8> = Vec::new();
			data.push(500);
			for i in range(0u, 500u){
				data.push(0x01);
			}
			conn.send_chunk_data(data);
			conn.send_level_finalize(10, 5, 10);
			
			conn.send_spawn_player(5, 3, 5, 5, 5);
			conn.send_pos(5, 3, 5, 5, 5);
			
			conn.send_ping();
		}
	}
	Ok(())
}

fn parse_packet(config: Configuration, mut conn: TcpStream) -> Packet{
	let packet_id = conn.read_byte().unwrap();
	let packet_len = match packet_id{
		0 => 130,
		_ => 1
	};
	let data = conn.read_exact(packet_len).unwrap();
	return Packet{
		packet_id: packet_id,
		packet_len: packet_len,
		data: data
	}
}

fn main(){
    let config = Configuration{
        address: "0.0.0.0".to_string(),
        port: 25565,
        max_players: 20,
        server_name: "RustServerBetaDontJoin".to_string(),
        server_motd: "A Minecraft classic server written in Rust!".to_string(),
        is_public: "True".to_string(),
        salt: "DEMOSALT12341".to_string(),
        heartbeat_interval: 45
    };
    let timer_config_clone = config.clone();
    spawn(proc() {
        loop{
            send_heartbeat(timer_config_clone.clone());
            timer::sleep(Duration::seconds(timer_config_clone.heartbeat_interval));
        }
    });
    let mut acceptor = TcpListener::bind(config.address.as_slice(), config.port).listen().unwrap();
    println!("Rustymine is listening on {}:{}", config.address, config.port);
    for connection in acceptor.incoming(){
		let clone = config.clone();
		spawn(proc() {
			handle_connection(clone, connection.unwrap());
		});
	}
}
