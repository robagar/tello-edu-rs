use tokio::{spawn, task};
use tokio::sync::mpsc;
use tokio::net::UdpSocket;
use bytebuffer::ByteBuffer;

use crate::errors::Result;

pub const VIDEO_WIDTH:u32 = 960;
pub const VIDEO_HEIGHT:u32 = 720; 

const VIDEO_UDP_PORT:u32 = 11111;
const MAX_CHUNK_SIZE:usize = 1460;


pub type TelloVideoSender = mpsc::UnboundedSender<TelloVideoFrame>;
pub type TelloVideoReceiver = mpsc::UnboundedReceiver<TelloVideoFrame>;

pub fn make_tello_video_channel() -> (TelloVideoSender, TelloVideoReceiver) {
    mpsc::unbounded_channel()
}

/// A frame of video from the drone.
#[derive(Debug)]
pub struct TelloVideoFrame {
    pub data: Vec<u8>
}

#[derive(Debug)]
pub(crate) struct VideoListener {
    task: task::JoinHandle<()>
}   

impl VideoListener {
    pub(crate) async fn start_listening(sender:TelloVideoSender) -> Result<Self> { 
        let local_address = format!("0.0.0.0:{VIDEO_UDP_PORT}");
        println!("[Video] START LISTENING at {local_address}");

        let sock = UdpSocket::bind(&local_address).await?;

        let task = spawn(async move {
            let mut buf = ByteBuffer::new();
            loop {
                let s = &sock;
                let mut chunk = vec![0; MAX_CHUNK_SIZE]; //Vec::with_capacity(MAX_CHUNK_SIZE);        
                let n = s.recv(&mut chunk).await.unwrap();
                if n != 0 {
                    buf.write_bytes(&chunk);

                    if n < MAX_CHUNK_SIZE {
                        let frame = TelloVideoFrame { data: buf.into_vec() };
                        sender.send(frame).unwrap();
                        buf = ByteBuffer::new();
                    }
                }
            }
        });

        Ok(Self { task })
    }

    pub(crate) async fn stop_listening(&self) -> Result<()> {
        println!("[Video] STOP LISTENING");
        self.task.abort();
        // TODO?
        // let _err = self.task.await;
        Ok(())
    }
 }
