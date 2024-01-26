use bytes::Bytes;
use mini_redis::client::{self};
use tokio::sync::{
    mpsc,
    oneshot::{self, Sender},
};

#[derive(Debug)]
enum Command {
    Get {
        key: String,
        resp: Responder<Option<Bytes>>,
    },
    Set {
        key: String,
        val: Bytes,
        resp: Responder<()>,
    },
}

type Responder<T> = Sender<mini_redis::Result<T>>;

#[tokio::main]
async fn main() {
    let (tx, mut rx) = mpsc::channel(32);
    let tx2 = tx.clone();

    let t1 = tokio::spawn(async move {
        let (one_tx, one_rx) = oneshot::channel();
        tx.send(Command::Get {
            key: "name".to_string(),
            resp: one_tx,
        })
        .await
        .unwrap();

        let res = one_rx.await;
        println!("t1 res,{:?}", res)
    });

    let t2 = tokio::spawn(async move {
        let (one_tx, one_rx) = oneshot::channel();
        tx2.send(Command::Set {
            key: "name".to_string(),
            val: "phil".into(),
            resp: one_tx,
        })
        .await
        .unwrap();
        let res = one_rx.await;
        println!("t2 res,{:?}", res);
    });

    let manager = tokio::spawn(async move {
        let mut client = client::connect("127.0.0.1:6379").await.unwrap();
        while let Some(message) = rx.recv().await {
            match message {
                Command::Get { key, resp } => {
                    let res = client.get(&key).await;
                    let _ = resp.send(res);
                }
                Command::Set { key, val, resp } => {
                    let res = client.set(&key, val).await;
                    let _ = resp.send(res);
                }
            }
        }
    });

    t1.await.unwrap();
    t2.await.unwrap();
    manager.await.unwrap();
}
