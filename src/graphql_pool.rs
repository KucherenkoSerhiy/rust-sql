use std::vec::Vec;
use std::str;
use std::io::prelude::*;
use std::convert::Into;
use std::option::Option;

use mysql;
use mio::{EventLoop, EventSet, PollOpt, Sender};
use eventual::Future;
use nom::IResult;

use reader;
use parser;
use serialize;
use deserialize;
use connection_pool::*;
use connection::*;
use def::*;

#[derive(Clone)]
pub struct GraphQLPool {
    pub sender: Sender<GraphqlMsg>,
}

impl GraphQLPool {
    pub fn new (db_conn: &str, db_name: &str, path_name: &str) -> GraphQLPool{

        let db = reader::extract_database_from_file(path_name);

        let pool = mysql::Pool::new(db_conn).unwrap();
        let mut conn = pool.get_conn().unwrap();

        conn.query(serialize::create_database(db_name.to_string())).unwrap();
        conn.query(serialize::use_database(db_name.to_string())).unwrap();
        for table in & db {
            conn.query((&(serialize::create_table(db_name.to_string(), &table))).to_string()).unwrap();
        }

        let mut targetPool = TargetPool{
            pool: pool.clone(),
            database: db.clone(),
            working_database_name: db_name.to_string(),
        };

        GraphQLPool{
            sender: ConnectionPool::new(targetPool.clone()),
        }
    }

    pub fn get (&self, query: &str) -> Future<String, ()> {
        let (tx, future) = Future::<String, ()>::pair();
        self.sender.send(GraphqlMsg::Request{
            operation: "get".to_string(),
            body: query.to_string(),
            tx: tx
        }).unwrap();
        future
    }


    pub fn add (&mut self, query: &str) -> Future<String, ()> {
        let (tx, future) = Future::<String, ()>::pair();
        self.sender.send(GraphqlMsg::Request{
            operation: "add".to_string(),
            body: query.to_string(),
            tx: tx
        }).unwrap();
        future
    }

    pub fn update (&mut self, query: &str) -> Future<String, ()> {
        let (tx, future) = Future::<String, ()>::pair();
        self.sender.send(GraphqlMsg::Request{
            operation: "update".to_string(),
            body: query.to_string(),
            tx: tx
        }).unwrap();
        future
    }

    pub fn delete (&mut self, query: &str) -> Future<String, ()> {
        let (tx, future) = Future::<String, ()>::pair();
        self.sender.send(GraphqlMsg::Request{
            operation: "delete".to_string(),
            body: query.to_string(),
            tx: tx
        }).unwrap();
        future
    }

    pub fn destroy_database (&mut self){
        //let mut conn = self.pool.get_conn().unwrap();
        //conn.query(serialize::destroy_database((&self.working_database_name).to_string())).unwrap();
    }
}