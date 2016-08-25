use eventual::{Future, Async, Complete};

use mio::*;
use mio::tcp::{TcpStream, TcpListener};
use mio::util::Slab;

use std::net::SocketAddr;
use std::borrow::Cow;
use std::error::Error;
use std::io;
use std::thread;
use std::fmt::Debug;
use std::vec::Vec;

use env_logger;

use connection::*;
use def::TargetPool;
use serialize;


pub const SERVER_TOKEN: Token = Token(0);

pub const NUMBER_OF_CONNECTIONS: usize = 20;

pub struct ConnectionPool {
    socket: TcpListener,
    token_counter: usize,
    connections: Slab<Connection>,
    request_messages: Vec<GraphqlMsg>,
    response_messages: Vec<GraphqlMsg>,
    target: TargetPool,
    serializer: serialize::Serializer
}

impl ConnectionPool {

    pub fn new(targetPool: TargetPool, serializer: serialize::Serializer) -> Sender<GraphqlMsg> {
        let addr: SocketAddr = "0.0.0.0:10000".parse::<SocketAddr>()
            .ok().expect("Failed to parse host:port string");
        let server_socket = TcpListener::bind(&addr).ok().expect("Failed to bind address");

        let mut event_loop = EventLoop::<ConnectionPool>::new().ok().expect("Failed to create event loop");
        event_loop.register(&server_socket,
                            SERVER_TOKEN,
                            EventSet::readable(),
                            PollOpt::edge()).unwrap();

        let mut pool = ConnectionPool {
            socket: server_socket,
            connections: Slab::new_starting_at(Token(1), 32768),
            token_counter: 1,
            request_messages: Vec::new(),
            response_messages: Vec::new(),
            target: targetPool,
            serializer: serializer
        };
        pool.create_connections(addr);

        let mut sender = event_loop.channel();

        thread::Builder::new().name("event_handler".to_string()).spawn(move || {
            event_loop.run(&mut pool).ok().expect("Failed to start event loop");
        });

        sender
    }

    fn create_connections(&mut self, addr: SocketAddr){
        let client_socket = TcpStream::connect(&addr).ok().expect("Failed to unwrap the socket");
    }

/*
    fn create_connection(&mut self, event_loop: &mut EventLoop<ConnectionPool>,address:&IpAddr) -> RCResult<Token>{
        //println!("[ConnectionPool::create_connection]");
        let mut conn = try_rc!(connect(SocketAddr::new(address.clone(),9042),
                                None,
                                event_loop,
                                self.event_handler.clone()),"Failed connecting");
        let token = try_rc!(self.add_connection(address.clone(),conn),"Failed adding a new connection");
        Ok(token)
    }
*/
    fn find_connection_by_token(&mut self, token: Token) -> Result<&mut Connection,&'static str>{
        println!("[ConnectionPool::find_connection_by_token]");
        if !self.connections.is_empty() {
            let conn = Ok(self.connections.get_mut(token).unwrap());
            //println!("Connection with {:?} -> {:?}",token,conn );
            return conn;
        }
        Err("There is no connection found")
    }

}

impl Handler for ConnectionPool {
    type Timeout = usize;
    type Message = GraphqlMsg;

    fn notify(&mut self, event_loop: &mut EventLoop<ConnectionPool>, msg: GraphqlMsg) {
        self.request_messages.push(msg);
        //self.ready(event_loop, SERVER_TOKEN, EventSet::writable());

    }

    fn ready(&mut self,
             event_loop: &mut EventLoop<ConnectionPool>,
             token: Token,
             events: EventSet
            )
    {
        match token {
            // A read event for our `Server` token means we are establishing a new connection.
            SERVER_TOKEN => {
                //println!("ConnectionPool::ready: got a server token");
                let client_socket = match self.socket.accept() {
                    Err(e) => {
                        println!("Accept error: {}", e);
                        return;
                    },
                    Ok(None) => unreachable!("Accept has returned 'None'"),
                    Ok(Some((client_socket, addr))) => client_socket
                };
                let new_token = Token(self.token_counter);
                self.token_counter += 1;
                let connection = Connection::new(client_socket, new_token, self.target.clone(), self.serializer.clone());

                self.connections.insert(connection);
                event_loop.register(&self.connections[new_token].socket, new_token,
                                    EventSet::writable(), PollOpt::oneshot()
                ).or_else(|e| {Err(e)});
            }
            token => {
                let mut connection = self.connections.get_mut(token).unwrap();
                //we're getting response
                if events.is_readable() {
                    //println!("ConnectionPool::ready: ready to read from client");
                    self.response_messages.append(&mut connection.get_responses());
                    event_loop.reregister(&connection.socket, connection.token, EventSet::writable(),
                                          PollOpt::edge() | PollOpt::oneshot()).unwrap();
                }
                if events.is_writable(){
                    //println!("ConnectionPool::ready: ready to write to client");
                    if !self.request_messages.is_empty(){
                        //write request to the client and reregister it to readable.
                        //println!("ConnectionPool::ready: have a message");
                        connection.pushRequest(self.request_messages.remove(0));
                        connection.process();
                        event_loop.reregister(&connection.socket, connection.token, EventSet::readable(),
                                              PollOpt::edge() | PollOpt::oneshot()).unwrap();

                    }
                    else {
                        //no requests, do nothing
                    }
                }
            }
        }
    }
}