use bytes::Bytes;
use mini_redis::client::{self};
use tokio::sync::mpsc;

#[derive(Debug)]
enum Command {
    Get { key: String },
    Set { key: String, val: Bytes },
}

#[tokio::main]
async fn main() {
    let (tx, mut rx) = mpsc::channel(32);
    let tx2 = tx.clone();

    let t1 = tokio::spawn(async move {
        tx.send(Command::Get {
            key: "name".to_string(),
        })
        .await
        .unwrap();
    });

    let t2 = tokio::spawn(async move {
        tx2.send(Command::Set {
            key: "name".to_string(),
            val: "phil".into(),
        })
        .await
        .unwrap();
    });

    let manager = tokio::spawn(async move {
        let mut client = client::connect("127.0.0.1:6379").await.unwrap();
        while let Some(message) = rx.recv().await {
            match message {
                Command::Get { key } => {
                    client.get(&key).await.unwrap();
                }
                Command::Set { key, val } => {
                    client.set(&key, val).await.unwrap();
                }
            }
        }
    });

    t1.await.unwrap();
    t2.await.unwrap();
    manager.await.unwrap();
}
