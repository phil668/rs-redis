use bytes::Bytes;
use mini_redis::client::{self, Message};
use tokio::sync::mpsc;

#[derive(Debug)]
enum Command {
    Get { key: String },
    Set { key: String, val: Bytes },
}

#[tokio::main]
async fn main() {
    // let mut client = client::connect("127.0.0.1:6379").await.unwrap();
    let (tx, mut rx) = mpsc::channel(32);
    let tx2 = tx.clone();

    tokio::spawn(async move {
        tx.send(Command::Get {
            key: "name".to_string(),
        })
        .await
        .unwrap();
    });

    tokio::spawn(async move {
        tx2.send(Command::Set {
            key: "name".to_string(),
            val: "phil".into(),
        })
        .await;
    });

    // while let Some(message) = rx.recv().await {
    //     println!("{}", message)
    // }
    let manager = tokio::spawn(async {
        let mut client = client::connect("127.0.0.1:6379").await.unwrap();
        while let Some(message) = rx.recv().await {
            match message {
                Command::Get { key } => {
                    client.get(key).await;
                }
                Command::Set { key, val } => {
                    client.set(key, value).await;
                }
            }
        }
    });
}
