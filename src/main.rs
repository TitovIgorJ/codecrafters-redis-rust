use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tracing::{error, info};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:6379").await?;

    while let Ok((stream, addr)) = listener.accept().await {
        info!("Accepted new connection: {}", addr);
        let (mut reader, mut writer) = tokio::io::split(stream);

        tokio::spawn(async move {
            let mut buf = vec![0; 1024];
            loop {
                match reader.read_buf(&mut buf).await {
                    Ok(0) => break,
                    Ok(n) => {
                        info!("Read {} bytes", n);
                        writer.write_all(b"+PONG\r\n").await.unwrap();
                        writer.flush().await.unwrap();
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
