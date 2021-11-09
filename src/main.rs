use tiny_http::{Server, Response, Request, Method, Header};
use redis;
use std::{time::Instant, io::{Error, ErrorKind, Cursor}, path::PathBuf, fs::File};
use json::{JsonValue, parse};
use mongodb::{sync::{Collection, Client}, options::{self, ClientOptions, UpdateModifications}};
use serde::{Serialize, Deserialize};
use bson::{doc, document::Document};

enum ResponseType {
    File(Response<File>),
    Curs(Response<Cursor<Vec<u8>>>),
}

#[derive(Debug, Serialize, Deserialize)]
struct Var {
    name: String,
    value: String,
}

fn main() {

    // tiny_http
    let server = Server::http("127.0.0.1:8000").unwrap();

    // Redis
    let redis_client = match redis::Client::open("redis://127.0.0.1:6379") {
        Ok(r) => r,
        Err(e) => {
            println!("Error opening redis client: {:?}", e);
            std::process::exit(1);
        }
    };
    let mut redis_con = match redis_client.get_connection() {
        Ok(r) => r,
        Err(e) => {
            println!("Error connecting to redis cache: {:?}", e);
            std::process::exit(1);
        }
    };

    // MongoDB
    let client_options = ClientOptions::parse("mongodb://user:password@localhost:27017").unwrap();
    let mongo_client = match Client::with_options(client_options) {
        Ok(c) => {
            println!("ok");
            c
        },
        Err(e) => {
            println!("Could not connect to mongodb: {:?}", e);
            std::process::exit(1);
        }
    };
    let mongo_db = mongo_client.database("main");
    let collection = mongo_db.collection::<Var>("variables");

    // Serve requests
    for mut request in server.incoming_requests() {
        let response: ResponseType = match request.url() {
            "/" => ResponseType::File(match root_handler(&request) {
                Err(e) => {
                    println!("Error providing index file: {:?}", e);
                    std::process::exit(1);
                },Ok(r) => r,
            }),
            "/cache" => ResponseType::Curs(match cache_handler(&mut request, &mut redis_con, &collection) {
                Err(e) => {
                    println!("Error communicating with cache: {:?}", e);
                    Response::from_string(format!("Error communicating with cache: {:?}", e))
                }, Ok(r) => r,
            }),
            // "/database" => ResponseType::File(match db_handler(&request) {
            //     Err(e) => {
            //         println!("Error communicating with database: {:?}", e);
            //         std::process::exit(1);
            //     }, Ok(r) => r,
            // }),
            "/css/styles.css" => ResponseType::File(match css_response() {
                Err(e) => {
                    println!("Could not load CSS file: {:?}", e);
                    std::process::exit(1);
                }, Ok(r) => r
            }),
            "/assets/main.js" => ResponseType::File(match js_response(){
                Err(e) => {
                    println!("Could not load JavaScript file: {:?}", e);
                    std::process::exit(1);
                }, Ok(r) => r
            }),
            _ => ResponseType::File(match html_response("html/404.html", 404) {
                Err(e) => {
                    println!("Error providing 404 file: {:?}", e);
                    std::process::exit(1);
                }, Ok(r) => r,
            }),
        };

        // respond
        match match response {
            ResponseType::File(mut f) => {
                f.add_header(Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap());
                f.add_header(Header::from_bytes(&b"Access-Control-Allow-Headers"[..], &b"*"[..]).unwrap());
                f.add_header(Header::from_bytes(&b"Access-Control-Request-Headers"[..], &b"*"[..]).unwrap());
                request.respond(f)
            },
            ResponseType::Curs(mut c) => {
                c.add_header(Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap());
                c.add_header(Header::from_bytes(&b"Access-Control-Allow-Headers"[..], &b"*"[..]).unwrap());
                c.add_header(Header::from_bytes(&b"Access-Control-Request-Headers"[..], &b"*"[..]).unwrap());
                request.respond(c)
            }
        } {
            Err(e) => {
                println!("Error responding to request: {:?}", e);
                std::process::exit(1);
            }, _ => {}
        };
    }

}

fn html_response(file_name: &str, status_code: i32) -> Result<Response<File>, Error> {
    let mut r = Response::from_file(get_file(file_name)?);
    r = r.with_status_code(status_code);
    r.add_header(Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..]).unwrap());
    Ok(r)
}

fn css_response() -> Result<Response<File>, Error> {
    let mut r = Response::from_file(get_file("css/styles.css")?);
    r = r.with_status_code(200);
    Ok(r)
}

fn js_response() -> Result<Response<File>, Error> {
    let mut r = Response::from_file(get_file("assets/main.js")?);
    r = r.with_status_code(200);
    Ok(r)
}

fn root_handler(req: &Request) -> Result<Response<File>, Error> {
    match req.method() {
        Method::Get => {
            Ok(html_response("html/index.html", 200)?)
        },
        _ => {
            Ok(html_response("html/404.html", 404)?)
        }
    }
}

// fn db_handler(_req: &Request) -> Result<Response<File>, Error> {
//     html_response("html/404.html", 404)
//     match req.method() {
//         Method::Post => Response::from_string("POST"),
//         Method::Get => Response::from_string("GET"),
//         _ => Response::from_string("Method not implemented"),
//     }
// }

fn cache_handler(req: &mut Request, con: &mut redis::Connection, col: &Collection<Var>) -> Result<Response<Cursor<Vec<u8>>>, Error> {
    let pre_cmd: functions::Function = parse_input(req)?;
    let cmd: redis::Cmd = pre_cmd.command()?;
    
    let cache_now = Instant::now();
    let cache_response: redis::Value = cmd.query(con).map_err(|_e| Error::new(ErrorKind::Other, "Could not apply command to cache"))?;
    let cache_elapsed = cache_now.elapsed();

    use functions::{Type::*, FunctionType::*};

    let (db_response, db_elapsed) = match pre_cmd.ftype {
        Set => {
            let filter: Document = doc! { "name": &pre_cmd.vname };
            let var: UpdateModifications = UpdateModifications::Document(doc! {
                "$set": {
                    "name": pre_cmd.vname,
                    "value": match pre_cmd.vtype.unwrap() {
                        Str(s) => s,
                        Int(i) => format!("{}", i),
                    },
                }
            });
            let update_op = options::FindOneAndUpdateOptions::builder().upsert(true).build();
            let db_now = Instant::now();
            (col.find_one_and_update(filter, var, update_op).map_err(|_e| {
                println!("{:?}", _e);
                Error::new(ErrorKind::Other, "Error finding and updating mongodb document")
            }),
            db_now.elapsed())
        },
        Get => {
            let filter: Document = doc! { "name": pre_cmd.vname };
            let db_now = Instant::now();
            
            (col.find_one(filter, None).map_err(|_e| {
                println!("{:?}", _e);
                Error::new(ErrorKind::Other, "Error finding mongodb document")
            }),
            db_now.elapsed())
        },
        Del => {
            let filter: Document = doc! { "name": pre_cmd.vname };
            let db_now = Instant::now();
            (col.find_one_and_delete(filter, None).map_err(|_e| Error::new(ErrorKind::Other, "Error finding and deleting mongodb document")),
            db_now.elapsed())
        },
    };
    let r = Response::from_string::<String>(
        format!("Cache response: {:?}\nTime taken: {:.2?}\nDB response: {:?}\nTime taken: {:.2?}",
            cache_response,
            cache_elapsed,
            db_response,
            db_elapsed).into());
    Ok(r)
}

fn get_file(name: &str) -> Result<File, Error> {
    let mut wd: PathBuf = std::env::current_dir()?;
    wd.push(name);
    Ok(File::open(wd)?)
}

fn parse_input(req: &mut Request) -> Result<functions::Function, Error> {
    let mut content = String::new();
    req.as_reader().read_to_string(&mut content)?;
    println!("{:?}", content);
    let json: JsonValue = match parse(&content) {
        Ok(j) => Ok(j),
        Err(_e) => {
            Err(Error::new(ErrorKind::Other, "Could not parse string to json"))
        }
    }?;

    let cmd_as_string: String = json["cmd"].to_string();
    let iter = cmd_as_string.split_whitespace();
    let cmd = functions::Function::from(iter);

    cmd
}

mod functions {
    use std::io::{Error, ErrorKind};
    use std::str::SplitWhitespace;
    use redis;

    pub struct Function {
        pub ftype: FunctionType,
        pub vname: String,
        pub vtype: Option<Type>,
    }

    pub enum Type {
        Str(String),
        Int(i64)
    }

    pub enum FunctionType {
        Set,
        Get,
        Del,
    }

    impl FunctionType {
        fn from_str(s: &str) -> Result<FunctionType, Error> {
            match s {
                "SET" => Ok(FunctionType::Set),
                "GET" => Ok(FunctionType::Get),
                "DEL" => Ok(FunctionType::Del),
                _ => Err(Error::new(ErrorKind::Other, "Command not understood"))
            }
        }

        fn to_str(&self) -> String {
            match self {
                FunctionType::Set => "SET".into(),
                FunctionType::Get => "GET".into(),
                FunctionType::Del => "DEL".into(),
            }
        }
    }

    impl Function {
        pub fn from(mut iter: SplitWhitespace) -> Result<Function, Error> {
            let f: FunctionType = FunctionType::from_str(match iter.next() {
                Some(s) => Ok(s.into()),
                None => Err(Error::new(ErrorKind::Other, "Could not read function type")),
            }?)?;
            match f {
                FunctionType::Set => {
                    let (vtype, vname): (Type, String) = vtype_from_str(&mut iter)?;
                    Ok(Function {
                        ftype: f,
                        vtype: Some(vtype),
                        vname: vname,
                    })
                },
                _ => match iter.next() {
                    Some(i) => Ok(Function { ftype: f, vname: i.into(), vtype: None }),
                    None => Err(Error::new(ErrorKind::Other, "Could not read variable name")),
                },
            }
        }

        pub fn command(&self) -> Result<redis::Cmd, Error> {
            let mut cmd = redis::Cmd::new();
            cmd.arg(self.ftype.to_str()).arg(&self.vname);
            match self.ftype {
                FunctionType::Set => {
                    cmd.arg(match &self.vtype {
                        Some(t) => match t {
                            Type::Str(s) => Ok(s.clone()),
                            Type::Int(i) => Ok(format!("{}", i)),
                        }, _ => Err(Error::new(ErrorKind::Other, "Could not parse variable value")),
                    }?);
                    cmd.arg("EX").arg(15);
                    Ok(cmd)
                },
                _ => Ok(cmd)
            }
        }
    }

    fn vname_from_str(s: Option<&str>) -> Result<&str, Error> {
        match s {
            Some(s) => Ok(s),
            None => Err(Error::new(ErrorKind::Other, "Name not found")),
        }
    }

    fn string_value(string: Option<&str>, i: &mut SplitWhitespace) -> Result<String, Error> {
        let s: &str = match string {
            Some(s) => Ok(s),
            None => Err(Error::new(ErrorKind::Other, "Could not understand value of string"))
        }?;

        let b: usize;
        let mut r: String;
        b = match s.find("\"") {
            Some(i) => Ok(i),
            None => Err(Error::new(ErrorKind::Other, "Could not understand value of string"))
        }?;

        if b < s.len() - 1 {
            if s[b+1..].contains("\"") {
                return Ok(s[b+1..(s[b+1..].find("\"").unwrap() - 1)].into());
            }
        }

        r = s[b+1..].into();

        let mut next: &str = match i.next() {
            Some(s) => Ok(s),
            None => Err(Error::new(ErrorKind::Other, "String may be missing a quotation mark")),
        }?;

        while !next.contains("\"") {
            r.push_str(" ");
            r.push_str(next);
            next = match i.next() {
                Some(s) => Ok(s.into()),
                None => Err(Error::new(ErrorKind::Other, "String may be missing a quotation mark")),
            }?;
        }
        let str_to_push: &str = &String::from(next)[..(next.find("\"").unwrap())];
        r.push_str(" ");
        r.push_str(str_to_push);
        Ok(r)
    }

    fn i64_value(i: &mut SplitWhitespace) -> Result<i64, Error> {
        match i.next() {
            Some(i) => i.parse::<i64>().map_err(|_e| Error::new(ErrorKind::Other, "Could not convert value to integer")),
            None => Err(Error::new(ErrorKind::Other, "Value not found")),
        }
    }

    fn vtype_from_str(i: &mut SplitWhitespace) -> Result<(Type, String), Error> {
        let type_as_str: Option<&str> = i.next();
        let vname: &str = vname_from_str(i.next())?;
        let vtype: Type = match type_as_str {
            Some(s) => match s {
                "int" => Ok(Type::Int(i64_value(i)?)),
                "string" => Ok(Type::Str(string_value(i.next(), i)?.to_string())),
                _ => Err(Error::new(ErrorKind::Other, "Type not found")),
            },
            None => Err(Error::new(ErrorKind::Other, "Type not found")),
        }?;
        Ok((vtype, vname.into()))
    }
}
