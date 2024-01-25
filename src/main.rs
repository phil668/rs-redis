use mini_redis::client;
use tokio::net::{TcpListener, TcpStream};

#[tokio::main]
async fn main() {
    let conn = TcpListener::bind("127.0.0.1:6379").await.unwrap();
}
