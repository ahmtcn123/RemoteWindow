use std::io::Write;
use minifb::{Key, Scale, Window, WindowOptions};
use std::convert::TryInto;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::{
    io::Read,
    net::TcpStream,
};


fn main() {
    const WIDTH: usize = 1920;
    const HEIGHT: usize = 1080;
    const CHUNK_SIZE : usize = 512;

    //Create ArcMutex for screen buffer
    let screen_buffer = Arc::new(Mutex::new(vec![0; WIDTH * HEIGHT]));
    //Create ArcAtomicBool for screen buffer update state
    let screen_updated = Arc::new(AtomicBool::new(true));

    let screen_updated_clone = screen_updated.clone();
    let screen_buffer_clone = screen_buffer.clone();

    let connection_thread = thread::spawn(move || {
        let mut capture_flag = true;
        let mut capture_buffer_size = false;
        let mut w = 0;
        let mut h = 0;

        let mut socket = TcpStream::connect("127.0.0.1:82").unwrap();
        // Notify server for frame render.
        socket.write(&[0x88]).unwrap();

        loop {
            // If server is ready to send the dimensions of the frame
            if capture_flag {
                let mut buffer = [0; 4];
                socket.read(&mut buffer).unwrap();
                if buffer[0] == 0x33 && buffer[1] == 0x34 && buffer[2] == 0x35 && buffer[3] == 0x36 {
                    capture_flag = false;
                    capture_buffer_size = true;
                }
            } else if capture_buffer_size {
                // Read the frame dimensions from the server
                let mut w_buffer = [0; 4];
                let mut h_buffer = [0; 4];
                socket.read(&mut w_buffer).unwrap();
                socket.read(&mut h_buffer).unwrap();
                w = u32::from_le_bytes(w_buffer);
                h = u32::from_le_bytes(h_buffer);
                capture_buffer_size = false;
                capture_flag = false; 
            } else {
                let mut rendered_pixel_count: usize = 0;

                //Read through the socket until the end of the frame
                while rendered_pixel_count < (h as usize * w as usize)  {
                    let mut pixel_buffer = [0; CHUNK_SIZE * 4];
                    // Read the next chunk of pixels from the server
                    socket.read(&mut pixel_buffer).unwrap();

                    // Write chunk as rgb entries to the screen buffer.
                    for chunk in 0..CHUNK_SIZE {
                        // 1920 * 1080 = 2073600
                        let u32_buffer : [u8; 4] = pixel_buffer[chunk * 4..(chunk + 1) * 4].try_into().unwrap();
                        if (h as usize * w as usize) > (rendered_pixel_count + chunk) {
                            screen_buffer_clone.lock().unwrap()[rendered_pixel_count + chunk] = u32::from_le_bytes(u32_buffer);
                        }
                    }
                    rendered_pixel_count += CHUNK_SIZE;
                }
                capture_flag = true;
                capture_buffer_size = false;
                // Notify server for new frame render.
                socket.write_all(&[0x88]).unwrap();
                screen_updated_clone.store(true, Ordering::Relaxed);
            }
        }
    });

    let screen_thread = thread::spawn(move || {
        let mut window = Window::new(
            "Test - ESC to exit",
            WIDTH,
            HEIGHT,
            WindowOptions {
                scale: Scale::FitScreen,
                scale_mode: minifb::ScaleMode::UpperLeft,
                resize: true,
                ..Default::default()
            },
        )
        .unwrap();

        while window.is_open() && !window.is_key_down(Key::Escape) {
            // If screen updated read from buffer.
            if screen_updated.load(Ordering::Relaxed) {
                match screen_buffer.try_lock() {
                    Ok(screen_buffer) => {
                        // Update buffer from window.
                        window.update_with_buffer(&screen_buffer, WIDTH, HEIGHT).unwrap();
                        screen_updated.store(false, Ordering::Relaxed);
                    }
                    Err(_) => {
                        println!("LOCK ERROR");
                    }
                }
            }
        }
    });

    // Join threads
    connection_thread.join().unwrap();
    screen_thread.join().unwrap();
}
