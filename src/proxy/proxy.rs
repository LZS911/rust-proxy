use std::{borrow::BorrowMut, net::TcpStream, sync::Mutex};
use std::{
    io::{prelude::*, ErrorKind},
    thread,
};
use std::{net::TcpListener, sync::Arc};

struct ProxyInner {
    t: i32,
}

pub struct Proxy {
    listener: TcpListener,
    inner: Arc<Mutex<ProxyInner>>,
}

impl ProxyInner {
    // 拷贝数据，用于转发浏览器和服务器之间的数据
    fn copy(
        inner: &mut Arc<Mutex<ProxyInner>>,
        from: &mut TcpStream,
        to: &mut TcpStream,
    ) -> std::io::Result<u64> {
        let mut len = 0;
        loop {
            {
                let mut t = inner.lock().unwrap();
                // t.t = 100;

                println!("----------------:{}", t.t);
            }

            let mut buf = [0; 10240];

            match from.read(&mut buf) {
                Ok(0) => {
                    return Ok(len);
                }
                Ok(bytes_read) => {
                    len += bytes_read as u64;
                    to.write(&buf[..bytes_read])?;
                    println!("read:{}", bytes_read);
                    continue;
                }
                Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(e) => {
                    return Err(e);
                }
            }
        }
    }
}
impl Proxy {
    pub fn new(listen_host: &str) -> Result<Self, String> {
        match TcpListener::bind(listen_host) {
            Ok(listener) => Ok(Proxy {
                listener,
                inner: Arc::new(Mutex::new(ProxyInner { t: 10 })),
            }),
            Err(e) => Err(e.to_string()),
        }
    }
    pub fn run(&mut self) {
        // 有请求过来就新开一个线程来处理
        for client in self.listener.incoming() {
            let client = client.unwrap();

            let mut inner_self = self.inner.borrow_mut().clone();
            thread::spawn(move || {
                Self::handle_connection(&mut inner_self, client);
            });
        }
    }

    fn handle_connection(inner: &mut Arc<Mutex<ProxyInner>>, mut client: TcpStream) {
        let mut buf = [0; 4096];
        client.read(&mut buf).unwrap();

        let req = String::from_utf8_lossy(&buf).to_string();
        // println!("str:{}", req);

        // 空格分隔获取请求方式，用于后面区分 CONNECT 和其他类型
        let data: Vec<&str> = req.split(" ").collect();
        let mut server;

        // https隧道代理
        if data[0] == "CONNECT" {
            // 连接目标服务器
            server = TcpStream::connect(data[1]).unwrap();
            println!("链接服务器{}, {}", data[0], data[1]);

            // 连接目标成功之后，返回下面内容，表示 通知浏览器连接成功
            client
                .try_clone()
                .unwrap()
                .write_all(b"HTTP/1.0 200 Connection Established\r\n\r\n")
                .unwrap();
        } else {
            server = TcpStream::connect("124.70.158.246:8889").unwrap();
            server.write(&buf).unwrap();
        }

        // 下面两个线程(主线程和子线程)分别转发 服务端=>浏览器 和  浏览器=>客户端 的数据
        let mut client1 = client.try_clone().unwrap();
        let mut server1 = server.try_clone().unwrap();

        let mut inner1 = inner.clone();
        let t1 = thread::spawn(move || {
            ProxyInner::copy(&mut inner1, &mut server1, &mut client1)
                .expect("服务端传输到客户端出错");
        });

        {
            let mut t = inner.lock().unwrap();
            t.t = 100;
        }
        ProxyInner::copy(inner, &mut client, &mut server).expect("客户端传输到服务端出错");

        t1.join().unwrap();
    }
}
