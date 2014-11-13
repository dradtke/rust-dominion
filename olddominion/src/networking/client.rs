use std::io::{BufferedStream, IoResult, TcpStream};

pub enum Status {
    NotConnected,
    Connected(BufferedStream<TcpStream>),
}

pub struct Client {
    name: &'static str,
    status: Status,
}

impl Client {
    pub fn new(name: &'static str) -> Client {
        Client{ name: name, status: NotConnected }
    }

    pub fn connect(&mut self, host: &'static str, port: u16) -> IoResult<()> {
        let conn = BufferedStream::new(try!(TcpStream::connect(host, port)));
        //conn.write_line("JOIN " + self.name);
        self.status = Connected(conn);
        Ok(())
    }
}
