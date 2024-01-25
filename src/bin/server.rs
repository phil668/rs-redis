use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use bytes::Bytes;

use mini_redis::{
    Command::{self, Get, Set},
    Connection, Frame,
};
use tokio::net::{TcpListener, TcpStream};

type DB = Arc<Mutex<HashMap<String, Bytes>>>;

#[tokio::main]
async fn main() {
    let conn = TcpListener::bind("127.0.0.1:6379").await.unwrap();

    println!("Redis Server listening at 127.0.0.1:6379");

    let db = Arc::new(Mutex::new(HashMap::new()));

    loop {
        let (socket, _) = conn.accept().await.unwrap();
        let db = Arc::clone(&db);
        tokio::spawn(async move {
            process(socket, db).await;
        });
    }
}

async fn process(stream: TcpStream, db: DB) {
    let mut connection = Connection::new(stream);
    while let Some(frame) = connection.read_frame().await.unwrap() {
        let response = match Command::from_frame(frame).unwrap() {
            Get(cmd) => {
                let db = db.lock().unwrap();
                if let Some(v) = db.get(cmd.key()) {
                    println!("GOT: {}", cmd.key());
                    Frame::Bulk(v.clone())
                } else {
                    Frame::Null
                }
            }
            Set(cmd) => {
                let mut db = db.lock().unwrap();
                let key = cmd.key();
                let value = cmd.value();
                println!("SET: {}-{:?}", cmd.key(), cmd.value());
                db.insert(key.to_string(), value.clone());
                Frame::Simple("Ok".to_string())
            }
            _ => {
                unimplemented!()
            }
        };
        connection.write_frame(&response).await.unwrap();
    }
}
