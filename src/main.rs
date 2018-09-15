#[macro_use]
extern crate lazy_static;

extern crate actix;
extern crate actix_web;
extern crate env_logger;
extern crate serde_json;
extern crate futures;

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate json;
use actix_web::http::{header, Method, StatusCode};
use actix_web::middleware::session::{self, RequestSession};
use actix_web::{
    error, fs, middleware, pred, server, App, Error, HttpRequest, HttpResponse, Path, Result, Json, HttpMessage, AsyncResponder
};
use futures::{Future, Stream};

use serde_json::{from_str, Value};
use json::JsonValue;
use std::sync::Mutex;
use std::collections::HashMap;
use std::{env, io};

#[derive(Debug, Serialize, Deserialize)]
struct MyObj {
        name: String,
        number: i32,
}

const MAX_SIZE: usize = 262_144; // max payload size is 256k

lazy_static! {
    static ref SERVERS: Mutex<HashMap<String, String>> = Mutex::new(HashMap::new());
}

fn add_server(addr: String, name: String) {
   SERVERS.lock().unwrap().insert(addr, name);
}

fn get_servers() -> HashMap<String, String> {
   SERVERS.lock().unwrap().clone()
}

fn make_json_string_of_servers() -> String {
   let server_map = get_servers();
   let mut server_json_string: String = "\"servers\": {".to_owned(); 
   let mut i = 0;
   for (addr, name) in &server_map {
       server_json_string.push('"');
       server_json_string.push_str(addr);
       server_json_string.push_str("\":\"");
       server_json_string.push_str(name);
       server_json_string.push('"');
       if(i<server_map.len()-1) {
           server_json_string.push(',');
       }
       i = i + 1;
   }
   println!("{}", server_map.len());
   server_json_string.push('}');

   //println!("{}", server_json_string);
   server_json_string
}

/// This handler manually load request payload and parse json-rust
fn json_endpoint(
    req: &HttpRequest,
) -> Box<Future<Item = HttpResponse, Error = Error>> {
    let address = parse_address_from_request(&req);
    req.payload()
        .concat2()
        .from_err()
        .and_then(|body| {
            // body is loaded, now we can deserialize json-rust
            let result = json::parse(std::str::from_utf8(&body).unwrap()); // return Result
            let mut _error = false;
            let injson: JsonValue = match result {
                Ok(v) => v,
                Err(e) => {
                _error = true;
                object!{"err" => e.to_string()
                } },
            };

            let mut response_string: String;

            if(_error) {
                response_string = injson.dump();
            } else {
                let server_name_json: Value = parse_server_name_from_json(&injson);
                println!("server name {}", server_name_json);


                //add_server(address, server_name);
                response_string = String::from("thanks!");
            }

            Ok(HttpResponse::Ok()
                .content_type("application/json")
                .body(response_string))
            })
        .responder()
}

fn parse_server_name_from_json(json: &JsonValue) -> Value {
    let json_string = json.dump();
    let json_value: Value = serde_json::from_str(&json_string).unwrap();
    let json_error_addserver: Value = serde_json::from_str("{\"err\":\"missing addserver\"}").unwrap();
    let json_error_name: Value = serde_json::from_str("{\"err\":\"missing name\"}").unwrap();

    let add_server_json_object = json_value.get("addserver");

    let mut error = false;
    let add_server_json_object: &Value = match &add_server_json_object {
        Some(v) => v,
        None => {
            error = true;
            &json_error_addserver
        },
    };

    let mut result_json : &Value = add_server_json_object;
    let mut error2 = false;
    if(!error) {
        let name_json = add_server_json_object.get("name");
        result_json = match name_json {
            Some(v) => v,
            None => {
                error2 = true;
                &json_error_name
                }
        };
    }
    let result_json: Value = result_json.clone();
    result_json
}

fn parse_address_from_request(req: &HttpRequest) -> String {
    let connection_info = req.connection_info();
    let remote_host_addr = connection_info.remote();
    let address = remote_host_addr.unwrap();
    let address_split: Vec<_> = address.split(':').collect();
    let address = address_split[0];
    println!("adress: {}", address);
    String::from(address)
}

fn welcome(req: &HttpRequest) -> Result<HttpResponse> {
    println!("{:?}", req);

    // session
    let mut counter = 1;
    if let Some(count) = req.session().get::<i32>("counter")? {
        println!("SESSION value: {}", count);
        counter = count + 1;
        req.session().set("counter", counter)?;
    } else {
        req.session().set("counter", counter)?;
    }

    Ok(HttpResponse::build(StatusCode::OK)
       .content_type("text/html; charset=utf-8")
       .body(include_str!("../static/welcome.html")))
}

fn with_param(req: &HttpRequest) -> HttpResponse {
    println!("{:?}", req);

    HttpResponse::Ok()
        .content_type("text/plain")
        .body(format!("Hello {}!", req.match_info().get("name").unwrap()))
}

fn p404(req: &HttpRequest) -> Result<fs::NamedFile> {
        Ok(fs::NamedFile::open("static/404.html")?.set_status_code(StatusCode::NOT_FOUND))
}

fn index(req: &HttpRequest) -> HttpResponse {
    let servers_string: String = make_json_string_of_servers();
    HttpResponse::Ok()
        .content_type("text/plain")
        .body(servers_string)
}

fn main() {
    env::set_var("RUST_LOG", "actix_web=debug");
    env::set_var("RUST_BACKTRACE", "1");
    env_logger::init();
    let sys = actix::System::new("server-manager-rust");

    add_server(String::from("8:8:8:8"), String::from("dummy server"));
    make_json_string_of_servers();

    let addr = server::new(
        || App::new()
            //enable logger
            .middleware(middleware::Logger::default())


            //.resource("", |r| r.method(Method::GET).f(index))
            .resource("", |r| r.f(index))
            .resource("/", |r| r.f(index))

            .resource("/json", |r| r.method(Method::POST).f(json_endpoint))
            
            .resource("/welcome", |r| r.f(welcome))
            .resource("/user/{name}", |r| r.method(Method::GET).f(with_param))

            .default_resource(|r| {
                // 404 for GET request
                r.method(Method::GET).f(p404);

                // all requests that are not GET
                //r.route().filter(pred::Not(pred::Get())).f(
                //    |req| HttpResponse::MethodNotAllowed());
            }))
    .bind("127.0.0.1:8080").expect("Can not bind to 127.0.0.1:8080")
    .shutdown_timeout(0)
    .start();

    println!("starting server");
    let _ = sys.run();
}
