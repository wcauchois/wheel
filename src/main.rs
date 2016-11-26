extern crate ansi_term;

use std::net::{TcpListener, TcpStream, SocketAddrV4};
use std::thread;
use std::collections::HashMap;
use std::io;
use std::io::{BufReader, BufWriter};
use std::io::prelude::*;
use ansi_term::Colour::*;

mod template;

// HTTP Spec: https://tools.ietf.org/html/rfc2616

#[derive(Debug)]
struct RequestInfo {
    method: String,
    path: String,
    headers: HashMap<String, String>,
}

struct ResponseInfo {
    content_type: String,
    body: String,
}

impl ResponseInfo {
    fn write_to_stream<W: io::Write>(self: &ResponseInfo, writer: &mut W) {
        write!(writer, "HTTP/1.1 200 OK\r\n\
            Content-Type: {}\r\n\
            Content-Length: {}\r\n\
            \r\n\
            {}",
            self.content_type,
            self.body.len(),
            self.body).unwrap();
    }
}

enum ParseState {
    ReadingRequestLine,
    ReadingHeaders
}

fn handle_client(stream: TcpStream) {
    thread::spawn(move || {
        let mut reader = BufReader::new(&stream);
        let mut buffer = String::new();
        let mut request_info = RequestInfo {
            method: String::new(),
            path: String::new(),
            headers: HashMap::new()
        };
        let mut parse_state = ParseState::ReadingRequestLine;
        while let Ok(_) = {
            buffer.clear();
            reader.read_line(&mut buffer)
        } {
            let buffer = buffer.trim(); // Hacky
            match parse_state {
                ParseState::ReadingRequestLine => {
                    for (i, part) in buffer.split_whitespace().enumerate() {
                        let part = part.to_string();
                        if i == 0 {
                            request_info.method = part;
                        } else if i == 1 {
                            request_info.path = part;
                        }
                        // Don't bother parsing the HTTP version
                    }
                    parse_state = ParseState::ReadingHeaders;
                }
                ParseState::ReadingHeaders => {
                    if buffer == "" {
                        break;
                    } else {
                        if let Some(idx) = buffer.find(':') {
                            let (key, value) = buffer.split_at(idx);
                            let value = &value[1..].trim_left();
                            request_info.headers.insert(key.to_string(), value.to_string());
                        }
                    }
                }
            }
        }
        // println!("Got request: {:?}", request_info);
        let mut writer = BufWriter::new(&stream);
        let response = ResponseInfo {
            content_type: "text/plain".to_string(),
            body: format!("You sent the request:\n{:?}", request_info)
        };
        response.write_to_stream(&mut writer);
    });
}

fn main() {
    let port = 1234;
    let listener = TcpListener::bind(
        SocketAddrV4::new("0.0.0.0".parse().unwrap(), port)
    ).unwrap();

    for tok in template::tokenize("foo bar {{ stringthing }} more text {% if for thing \"string\" 123 %}".chars()) {
    //for tok in template::tokenize("foo bar {{ stringthing }} more text".chars()) {
        println!("{:?}", tok);
    }
    //println!("{:?}", template::tokenize("foo bar".chars()).collect());
    panic!("quitting");

    println!("{}", Green.paint(format!("Listening on port {}", port)));

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                handle_client(stream);
            }
            Err(e) => {
                println!("Error connecting client: {}", e);
            }
        }
    }

    println!("Hello, world!");
}

