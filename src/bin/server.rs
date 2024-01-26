use std::{
    collections::HashMap,
    io::Cursor,
    sync::{Arc, Mutex},
};

use bytes::{Buf, Bytes, BytesMut};

use mini_redis::{
    Command::{self, Get, Set},
    Frame, Result,
};
use tokio::{
    io::{self, AsyncReadExt, AsyncWriteExt, BufWriter},
    net::{TcpListener, TcpStream},
};

type DB = Arc<Mutex<HashMap<String, Bytes>>>;

struct Connection {
    stream: BufWriter<TcpStream>,
    buffer: BytesMut,
}

impl Connection {
    fn new(stream: TcpStream) -> Self {
        Connection {
            stream: BufWriter::new(stream),
            buffer: BytesMut::with_capacity(4096),
        }
    }

    async fn read_frame(&mut self) -> Result<Option<Frame>> {
        loop {
            // 从buffer中读取出frame
            if let Some(frame) = self.parse_frame()? {
                return Ok(Some(frame));
            };
            // 如果buffer中读取不出来完整的frame，就要从stream中去获取
            let res = self.stream.read_buf(&mut self.buffer).await.unwrap();
            // 如果读取的字节数为0的话说明已经读取完毕了
            if res == 0 {
                if self.buffer.is_empty() {
                    return Ok(None);
                } else {
                    return Err("connection reset by peer".into());
                }
            }
        }
    }

    async fn write_frame(&mut self, frame: Frame) -> io::Result<()> {
        // todo
        match frame {
            Frame::Simple(v) => {
                self.stream.write_u8(b'+').await?;
                self.stream.write_all(v.as_bytes()).await?;
                self.stream.write_all(b"\r\n").await?;
            }
            Frame::Integer(v) => {
                self.stream.write_u8(b'-').await?;
                self.write_decimal(v).await?;
            }
            Frame::Error(err) => {
                self.stream.write_u8(b'-').await?;
                self.stream.write_all(err.as_bytes()).await?;
                self.stream.write_all(b"\r\n").await?;
            }
            Frame::Null => self.stream.write_all(b"$-1\r\n").await?,
            Frame::Bulk(v) => {
                let len = v.len();
                self.stream.write_u8(b'$').await?;
                self.write_decimal(len as u64).await?;
                self.stream.write_all(&v).await?;
                self.stream.write_all(b"\r\n").await?;
            }
            Frame::Array(_) => unimplemented!(),
        }
        self.stream.flush().await?;
        Ok(())
    }

    // 从buffer中解析出frame
    fn parse_frame(&mut self) -> Result<Option<Frame>> {
        let mut buf = Cursor::new(&self.buffer[..]);
        // 检查是否有足够读取出一个frame的buffer
        match Frame::check(&mut buf) {
            Ok(_) => {
                let len = buf.position() as usize;
                buf.set_position(0);
                let frame = Frame::parse(&mut buf)?;
                // 解析出frame之后，需要将buffer内对应的区域清空
                self.buffer.advance(len);
                Ok(Some(frame))
            }
            Err(mini_redis::frame::Error::Incomplete) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    // 将数字写入stream流内
    async fn write_decimal(&mut self, val: u64) -> io::Result<()> {
        use std::io::Write;

        let mut buf = [0u8; 12];
        let mut buf = Cursor::new(&mut buf[..]);
        write!(buf, "{}", val)?;
        let pos = buf.position() as usize;
        self.stream.write_all(&buf.get_ref()[..pos]).await?;
        self.stream.write_all(b"\r\n").await?;
        Ok(())
    }
}

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
