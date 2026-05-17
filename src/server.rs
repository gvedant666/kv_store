use crate::protocol::{parse_request, Command, ErrorCode, ResponseBuilder};
use crate::storage::StorageEngine;
use bytes::{Buf, BytesMut};
use mio::net::{TcpListener, TcpStream};
use mio::{Events, Interest, Poll, Token};
use std::collections::HashMap;
use std::io::{self, Read, Write};

const SERVER: Token = Token(0);
const READ_BUF_SIZE: usize = 4096;

struct Connection {
    stream: TcpStream,
    read_buf: BytesMut,
    write_buf: BytesMut,
}

impl Connection {
    fn new(stream: TcpStream) -> Self {
        Self {
            stream,
            read_buf: BytesMut::with_capacity(READ_BUF_SIZE),
            write_buf: BytesMut::new(),
        }
    }
}

pub struct Server {
    listener: TcpListener,
    poll: Poll,
    connections: HashMap<Token, Connection>,
    next_token: usize,
    storage: StorageEngine,
}

impl Server {
    pub fn bind(addr: &str) -> io::Result<Self> {
        let mut listener = TcpListener::bind(addr.parse().unwrap())?;
        let poll = Poll::new()?;
        
        poll.registry().register(
            &mut listener,
            SERVER,
            Interest::READABLE,
        )?;

        Ok(Self {
            listener,
            poll,
            connections: HashMap::new(),
            next_token: 1,
            storage: StorageEngine::new(),
        })
    }

    pub fn run(&mut self) -> io::Result<()> {
        let mut events = Events::with_capacity(1024);

        loop {
            // Poll blocks until an event occurs (like C's epoll_wait)
            self.poll.poll(&mut events, None)?;

            for event in events.iter() {
                match event.token() {
                    SERVER => loop {
                        match self.listener.accept() {
                            Ok((mut stream, _)) => {
                                let token = Token(self.next_token);
                                self.next_token += 1;

                                self.poll.registry().register(
                                    &mut stream,
                                    token,
                                    Interest::READABLE | Interest::WRITABLE,
                                )?;

                                self.connections.insert(token, Connection::new(stream));
                            }
                            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => break,
                            Err(e) => eprintln!("Accept error: {}", e),
                        }
                    },
                    token => {
                        let mut closed = false;
                        if event.is_readable() {
                            closed = !self.handle_read(token);
                        }
                        if !closed && event.is_writable() {
                            closed = !self.handle_write(token);
                        }

                        if closed {
                            self.connections.remove(&token);
                        }
                    }
                }
            }
        }
    }

    fn handle_read(&mut self, token: Token) -> bool {
        let mut buf = [0u8; READ_BUF_SIZE];

        loop {
            let read_result = {
                let conn = self.connections.get_mut(&token).unwrap();
                conn.stream.read(&mut buf)
            };

            match read_result {
                Ok(0) => return false, // Connection closed
                Ok(n) => {
                    {
                        let conn = self.connections.get_mut(&token).unwrap();
                        conn.read_buf.extend_from_slice(&buf[0..n]);
                    }
                    self.process_buffer(token);
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => return true,
                Err(_) => return false,
            }
        }
    }

    fn process_buffer(&mut self, token: Token) {
        // Parse using an immutable borrow of the connection's read buffer so we can
        // call process_command (which borrows self) without conflicting with an
        // outstanding mutable borrow of the connection.
        while let Ok(Some((cmd, consumed))) = {
            let read_buf = &self.connections.get(&token).unwrap().read_buf;
            parse_request(read_buf)
        } {
            let response = self.process_command(&cmd);
            let conn = self.connections.get_mut(&token).unwrap();
            conn.write_buf.extend_from_slice(&response);
            conn.read_buf.advance(consumed); // Idiomatic zero-copy buffer shift
        }
    }

    fn handle_write(&mut self, token: Token) -> bool {
        let conn = self.connections.get_mut(&token).unwrap();
        
        if conn.write_buf.is_empty() {
            return true;
        }

        match conn.stream.write(&conn.write_buf) {
            Ok(n) => {
                conn.write_buf.advance(n);
                true
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => true,
            Err(_) => false,
        }
    }

    // (Keep the exact process_command function from the previous server.rs file here)
    fn process_command(&self, cmd: &Command) -> Vec<u8> {
        let mut response = ResponseBuilder::new();
        // ... previous routing logic ...
        response.finish()
    }
}