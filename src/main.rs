use crate::command::bulk_string_array;
use bytes::{Buf, BytesMut};
use nom::AsBytes;
use std::collections::VecDeque;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tracing::{error, info};

pub mod command;
pub mod types;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();
    let listener = TcpListener::bind("127.0.0.1:6379").await?;
    while let Ok((stream, addr)) = listener.accept().await {
        info!("Accepted new connection: {}", addr);
        let (mut reader, mut writer) = tokio::io::split(stream);
        tokio::spawn(async move {
            let mut buf = BytesMut::with_capacity(64);
            loop {
                match reader.read_buf(&mut buf).await {
                    Ok(0) => break,
                    Ok(_n) => {
                        if buf.get_u8() == b'*' {
                            let (_, array) = bulk_string_array(&buf).unwrap();
                            let mut args = VecDeque::from_iter(array);

                            if let Some(c) = args.pop_front() {
                                match &c.value.to_ascii_lowercase()[..] {
                                    b"ping" => {
                                        if let Some(arg) = args.pop_front() {
                                            let v = arg.value;
                                            let len = v.len();
                                            let _ = writer.write(format!("${len}\r\n").as_bytes()).await.unwrap();
                                            let _ = writer.write(v.as_bytes()).await.unwrap();
                                            let _ = writer.write(b"\r\n").await.unwrap();
                                        } else {
                                            let _ = writer.write(b"+PONG\r\n").await.unwrap();
                                        }
                                    }
                                    b"echo" => {
                                        if args.len() == 1 {
                                            let arg = args.pop_front().unwrap();
                                            let v = arg.value;
                                            let len = v.len();
                                            let _ = writer.write(format!("${len}\r\n").as_bytes()).await.unwrap();
                                            let _ = writer.write(v.as_bytes()).await.unwrap();
                                            let _ = writer.write(b"\r\n").await.unwrap();
                                        } else {
                                            let _ = writer.write(b"+ERROR\r\n").await.unwrap();
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }

                    }
                    Err(err) => {
                        error!("{err}");
                        break;
                    }
                }
            }
        });
    }
    Ok(())
}
