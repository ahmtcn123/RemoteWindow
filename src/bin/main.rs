
use captrs::*;
use core::time::Duration;
use std::fs::File;
use std::io::ErrorKind::WouldBlock;
use std::io::Write;
use std::net::{Shutdown, TcpListener, TcpStream};
use std::{thread, time};

fn handle_client(mut stream: TcpStream) {
    let mut capturer = Capturer::new(0).unwrap();
    let (w, h) = capturer.geometry();
    let size = w as u64 * h as u64;
    println!("START: {}x{}", w, h);
    'test: loop {
        let (mut x, mut y) = (1, 1);
        let ps = capturer.capture_frame().unwrap();
        let (mut tot_r, mut tot_g, mut tot_b) = (0, 0, 0);

        for Bgr8 { r, g, b, .. } in ps.into_iter() {
            if x == 256 {
                if y == 128 {
                    //println!("OK, {}x{}", x, y);
                    let to_be_sent = format!("{},{},{}\n", r, g, b);
                    let c = stream.write_all(to_be_sent.as_bytes());
                    if let Ok(_) = c {
                        //println!("SENT {}", to_be_sent);
                    } else if let Err(_) = c {
                        stream.shutdown(Shutdown::Both).unwrap();
                        break 'test;
                        println!("ERR");
                    }
                    y = 1;
                    x = 1;
                } else {
                    //println!("X COMPLETE {}x{}", x, y);
                    y += 1;
                    x = 1;
                }
            } else {
                x += 1;
            }
            tot_r += r as u64;
            tot_g += g as u64;
            tot_b += b as u64;
        }
        tot_r = tot_r / size;
        tot_g = tot_g / size;
        tot_b = tot_b / size;
        //thread::sleep(Duration::from_millis(10));
    }
}

fn main() {
    let listener = TcpListener::bind("localhost:80");
    if let Ok(socket) = listener {
        for stream in socket.incoming() {
            match stream {
                Err(e) => println!("Accept err {}", e),
                Ok(stream) => {
                    thread::spawn(|| handle_client(stream));
                }
            }
            //handle_client(stream.unwrap());
        }
    } else if let Err(error) = listener {
        println!("Failed to bind server: {:#?}", error);
    }
}
