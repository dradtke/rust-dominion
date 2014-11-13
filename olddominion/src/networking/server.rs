use regex::Regex;
use std::comm;
use std::io::{Acceptor, BufferedStream, EndOfFile, Listener, TcpStream, TcpListener};

pub enum Status {
    WaitingForPlayers
}

pub struct ClientStub {
    name: String,
    conn: BufferedStream<TcpStream>,
}

pub struct Server {
    clients: Vec<ClientStub>,
    status: Status,
    conns: (Sender<()>, Receiver<TcpStream>),

    join_re: Regex,
}

impl Server {
    /// Creates a new server listening on the given host and port.
    ///
    /// The new server is initialized with an empty client list, a status of
    /// `WaitingForPlayers`, quit-signal send and incoming connection receive
    /// channels, and cached regexes. Any connections accepted are added to
    /// the channel to be received and turned into a `ClientStub` by methods
    /// like `wait_for_clients()`. Sending a unit value on the quit-signal
    /// send channel causes the server to hang up and stop listening for new
    /// connections.
    pub fn new(host: &'static str, port: u16) -> Result<Server, String> {
        let listener = match TcpListener::bind(host, port) {
            Ok(x) => x,
            Err(e) => return Err(format!("failed to bind to {}:{}: {}", host, port, e))
        };
        let mut acceptor = match listener.listen() {
            Ok(x) => x,
            Err(e) => return Err(format!("failed to start listening on {}:{}: {}", host, port, e))
        };

        let (cs, cr) = comm::channel(); // connections
        let (qs, qr) = comm::channel(); // quit notification

        let ac = acceptor.clone(); // acceptor closer
        spawn(proc() {
            let mut ac = ac;
            qr.recv();
            assert!(ac.close_accept().is_ok());
        });

        spawn(proc() {
            for conn in acceptor.incoming() {
                match conn {
                    Ok(conn) => cs.send(conn),
                    Err(ref e) if e.kind == EndOfFile => break,
                    Err(e) => println!("connection failed: {}", e),
                }
            }
            drop(cs);
        });

        let join_re = regex!("^JOIN (.*)$");

        Ok(Server{
            clients: Vec::new(),
            status: WaitingForPlayers,
            conns: (qs, cr),
            join_re: join_re,
        })
    }

    /// Checks if there is a client immediately available on the channel,
    /// and if so, adds it to the list.
    pub fn check_for_client(&mut self) {
        if let Ok(conn) = self.conns.ref1().try_recv() {
            self.add_client(conn);
        }
    }

    /// Continuously waits for clients until the channel hangs up.
    pub fn wait_for_clients(&mut self) {
        'waiting: loop {
            let incoming = {
                let (_, ref cr) = self.conns;
                match cr.recv_opt() {
                    Ok(conn) => Some(conn),
                    Err(_) => None,
                }
            };

            match incoming {
                Some(conn) => { self.add_client(conn); }, // TODO: do something with this return value
                None => break 'waiting,
            }
        }
    }

    /// Adds a client to the server's list of clients.
    ///
    /// It converts the given stream into a buffered stream, then reads the
    /// first line to make sure it matches the pattern "JOIN <name>". If it
    /// does, then the name is extracted and a new `ClientStub` is added to
    /// the list. Otherwise nothing happens.
    fn add_client(&mut self, conn: TcpStream) -> Result<(), String> {
        let mut conn = BufferedStream::new(conn);
        let welcome = match conn.read_line() {
            Ok(x) => x, Err(e) => return Err(format!("failed to read client's greeting: {}", e))
        };
        let captures = match self.join_re.captures(welcome.as_slice()) {
            Some(x) => x,
            None => return Err(format!("client's greeting did not match expected format: {}", welcome)),
        };
        let name = String::from_str(captures.at(1));
        self.clients.push(ClientStub{name: name, conn: conn});
        Ok(())
    }
}
