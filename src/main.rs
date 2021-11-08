use tiny_http::{Server, Response, Request, Method, Header};
use redis;
use std::{io::{Error, ErrorKind}, path::PathBuf, fs::File};
use json::{JsonValue, parse};

fn main() -> Result<(), redis::RedisError>{

    let server = Server::http("127.0.0.1:8000").unwrap();

    let redis_client = redis::Client::open("redis://127.0.0.1:6379")?;
    let mut _redis_con = redis_client.get_connection()?;

    for mut request in server.incoming_requests() {
        let response: Result<Response<File>, Error> = match request.url() {
            "/" => root_handler(&request),
            "/cache" => cache_handler(&mut request),
            "/database" => db_handler(&request),
            "/css/styles.css" => css_response(),
            "/assets/main.js" => js_response(),
            _ => html_response("html/404.html", 404),
        };


        let _ = request.respond(response.unwrap_or(
            match html_response("html/404.html", 404) {
                Err(e) => {
                    println!("Error: {:?}", e);
                    std::process::exit(1);
                }, Ok(f) => f
            }
        ));
    }

    Ok(())
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
            html_response("html/index.html", 200)
        },
        _ => {
            html_response("html/404.html", 404)
        }
    }
}

fn db_handler(_req: &Request) -> Result<Response<File>, Error> {
    html_response("html/404.html", 404)
    // match req.method() {
    //     Method::Post => Response::from_string("POST"),
    //     Method::Get => Response::from_string("GET"),
    //     _ => Response::from_string("Method not implemented"),
    // }
}

fn cache_handler(req: &mut Request) -> Result<Response<File>, Error> {
    let cmd: redis::Cmd = parse_input(req)?;
    html_response("html/404.html", 404)
    // match req.method() {
    //     Method::Post => Response::from_string("POST"),
    //     Method::Get => Response::from_string("GET"),
    //     _ => Response::from_string("Method not implemented"),
    // }
}

fn increment(con: &mut redis::Connection) -> redis::RedisResult<u64> {
    let r : u64 = redis::cmd("INCR").arg("visit_count").query(con)?;

    Ok(r)
}

fn get_file(name: &str) -> Result<File, Error> {
    let mut wd: PathBuf = std::env::current_dir()?;
    wd.push(name);
    Ok(File::open(wd)?)
}

fn parse_input(req: &mut Request) -> Result<redis::Cmd, Error> {
    let mut content = String::new();
    req.as_reader().read_to_string(&mut content)?;
    let json: JsonValue = match parse(&content) {
        Ok(j) => Ok(j),
        Err(e) => {
            Err(Error::new(ErrorKind::Other, "Could not parse string to json"))
        }
    }?;
    
    let mut cmd: redis::Cmd = redis::Cmd::new();

    let cmd_as_string: String = json["cmd"].to_string();
    let mut iter = cmd_as_string.split_whitespace();
    return match iter.next() {
        Some("SET") => Ok(functions::set(iter)?),
        Some("GET") => Ok(functions::get(iter)?),
        Some("DEL") => Ok(functions::del(iter)?),
        _ => Err(Error::new(ErrorKind::Unsupported, "Command not understood/not supported")),
    };
    
}

mod functions {

    use std::io::Error;
    use std::str::SplitWhitespace;
    use redis;

    pub fn get(iter: SplitWhitespace) -> Result<redis::Cmd, Error> {
        println!("getgetget");
        for item in iter {
            println!("{:?}", item);
        }
        Ok(redis::Cmd::new())
    }
    pub fn set(iter: SplitWhitespace) -> Result<redis::Cmd, Error> {
        println!("setset");
        Ok(redis::Cmd::new())
    }
    pub fn del(iter: SplitWhitespace) -> Result<redis::Cmd, Error> {
        println!("del");
        Ok(redis::Cmd::new())
    }
}
