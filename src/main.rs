use crate::command::bulk_string_array;
use crate::types::BulkString;
use bytes::{Buf, BytesMut};
use nom::AsBytes;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use tokio::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt, WriteHalf};
use tokio::net::{TcpListener, TcpStream};
use tracing::{error, info};

pub mod command;
pub mod types;

#[derive(Debug, Clone)]
struct Storage {
    inner: Arc<Mutex<HashMap<BulkString, BulkString>>>,
}

impl Storage {
    fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::default())),
        }
    }

    fn set(&self, key: BulkString, value: BulkString) {
        let mut lock = self.inner.lock().unwrap();
        lock.insert(key, value);
    }

    fn get(&self, key: &BulkString) -> Option<BulkString> {
        let lock = self.inner.lock().unwrap();
        let r = lock.get(key);
        r.cloned()
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();
    let listener = TcpListener::bind("127.0.0.1:6379").await?;

    let store = Storage::new();

    while let Ok((stream, addr)) = listener.accept().await {
        info!("Accepted new connection: {}", addr);
        let (mut reader, mut writer) = tokio::io::split(stream);
        let store_clone = store.clone();
        tokio::spawn(async move {
            let mut buf = BytesMut::with_capacity(4092);
            loop {
                match reader.read_buf(&mut buf).await {
                    Ok(0) => {
                        println!("Read 0 bytes");
                        return;
                    },
                    Ok(n) => {
                        if buf.get_u8() == b'*' {
                            let (remaining, array) = bulk_string_array(&buf[..]).unwrap();

                            buf.advance(buf.len() - remaining.len());

                            let mut args = VecDeque::from_iter(array);

                            if let Some(c) = args.pop_front() {
                                match &c.value.to_ascii_lowercase()[..] {
                                    b"ping" => {
                                        if let Some(arg) = args.pop_front() {
                                            let v = arg.value;
                                            let len = v.len();
                                            let _ = writer
                                                .write(format!("${len}\r\n").as_bytes())
                                                .await
                                                .unwrap();
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
                                            let _ = writer
                                                .write(format!("${len}\r\n").as_bytes())
                                                .await
                                                .unwrap();
                                            let _ = writer.write(v.as_bytes()).await.unwrap();
                                            let _ = writer.write(b"\r\n").await.unwrap();
                                        } else {
                                            send_error_bulk_str(&mut writer).await.unwrap();
                                        }
                                    }
                                    b"set" => {
                                        if args.len() >= 2 {
                                            let key = args.pop_front().expect("we checked length");
                                            let value =
                                                args.pop_front().expect("we checked length");
                                            store_clone.set(key, value);
                                            let _ = writer.write(b"+OK\r\n").await.unwrap();
                                        } else {
                                            send_error_bulk_str(&mut writer).await.unwrap();
                                        }
                                    }
                                    b"get" => {
                                        if args.is_empty() {
                                            send_error_bulk_str(&mut writer).await.unwrap();
                                        } else {
                                            let key = args.pop_front().expect("we checked length");
                                            match store_clone.get(&key) {
                                                Some(value) => {
                                                    send_bulk_str(&mut writer, &value).await.unwrap();
                                                }
                                                None => {
                                                    send_null_bulk_str(&mut writer).await.unwrap();
                                                }
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    Err(err) => {
                        error!("{err}");
                        return;
                    }
                }
            }
        });
    }
    Ok(())
}

async fn send_null_bulk_str(w: &mut WriteHalf<TcpStream>) -> io::Result<()> {
    w.write_all(b"-1\r\n").await
}

async fn send_error_bulk_str(w: &mut WriteHalf<TcpStream>) -> io::Result<()> {
    w.write_all(b"+ERROR\r\n").await
}

async fn send_bulk_str(w: &mut WriteHalf<TcpStream>, str: &BulkString) -> io::Result<()> {
    let len = str.value.len();
    w
        .write_all(format!("${len}\r\n").as_bytes())
        .await
        .unwrap();
    w.write_all(str.value.as_bytes()).await?;

    w.write_all(b"\r\n").await
}
