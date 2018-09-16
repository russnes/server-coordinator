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
use actix_web::http::{Method, StatusCode};
use actix_web::{
    fs, middleware, server, App, Error, HttpRequest, HttpResponse, Result, HttpMessage, AsyncResponder
};
use futures::{Future, Stream};
use serde_json::Value;
use json::JsonValue;
use std::sync::Mutex;
use std::collections::HashMap;
use std::{env, thread, time};
use std::net::{TcpStream, Shutdown};

#[derive(Debug, Serialize, Deserialize)]
struct MyObj {
        name: String,
        number: i32,
}

lazy_static! {
    static ref SERVERS: Mutex<HashMap<String, String>> = Mutex::new(HashMap::new());
    static ref ALIVE: Mutex<HashMap<String, bool>> = Mutex::new(HashMap::new());
}

fn add_server(addr: String, name: String) {
    let name = str::replace(&name, "\"", "");
   SERVERS.lock().unwrap().insert(addr, name);
}

fn get_servers() -> HashMap<String, String> {
   SERVERS.lock().unwrap().clone()
}

fn clear_servers() {
    SERVERS.lock().unwrap().clear();
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
       if i<server_map.len()-1 {
           server_json_string.push(',');
       }
       i = i + 1;
   }
   server_json_string.push('}');
   server_json_string
}

/// This handler manually loads request payload and parses json-rust
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

            if _error {
                response_string = injson.dump();
            } else {
                let server_name_json_with_possible_error: Value = parse_server_name_from_json(&injson);
                let server_name_json = server_name_json_with_possible_error.get("name");
                let mut was_ok = true;
                let server_name_json : &Value = match server_name_json {
                    Some(v) => v,
                    None => {
                        was_ok = false;
                        &server_name_json_with_possible_error
                        }
                };
                if was_ok {
                    //response_string = String::from("thanks for the server!");
                    response_string = String::from("OK");
                    if server_name_json.is_string() {
                        let connection_result = test_connection(&address);
                        if connection_result {
                            add_server(address, server_name_json.to_string())
                        } else {
                            //let json_error_name: Value = serde_json::from_str("{\"err\":\"can't connect\"}").unwrap();
                            //response_string = json_error_name.to_string();
                            response_string = String::from("NOT OK");
                        }
                    } else {
                        //let json_error_name: Value = serde_json::from_str("{\"err\":\"name is not string\"}").unwrap();
                        //response_string = json_error_name.to_string();
                        response_string = String::from("NOT OK");
                    }
                } else {
                    //response_string = server_name_json_with_possible_error.to_string();
                    response_string = String::from("NOT OK");
                }
            }

            Ok(HttpResponse::Ok()
                .content_type("application/json")
                .body(response_string))
            })
        .responder()
}

fn test_connection(address: &String) -> bool {
    let mut address_with_port = address.clone();
    address_with_port.push_str(":4476");
    let stream = TcpStream::connect(address_with_port);
    if let Ok(stream) = stream {
        stream.shutdown(Shutdown::Both).expect("shutdown call failed");
        true
    } else {
        false
    }
}

fn parse_server_name_from_json(json: &JsonValue) -> Value {
    let json_error_addserver: Value = serde_json::from_str("{\"err\":\"missing addserver\"}").unwrap();
    let json_error_name: Value = serde_json::from_str("{\"err\":\"missing name\"}").unwrap();

    let json_string = json.dump();
    let json_value: Value = serde_json::from_str(&json_string).unwrap();
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
    if !error {
        let name_json = add_server_json_object.get("name");
        result_json = match name_json {
            Some(v) => add_server_json_object,
            None => &json_error_name
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

fn p404(req: &HttpRequest) -> Result<fs::NamedFile> {
        Ok(fs::NamedFile::open("static/404.html")?.set_status_code(StatusCode::NOT_FOUND))
}

fn index(req: &HttpRequest) -> HttpResponse {
    let servers_string: String = make_json_string_of_servers();
    let address = parse_address_from_request(&req);
    let mut response_string = "{\"yourip\":\"".to_owned();
    response_string.push_str(&address);
    response_string.push_str("\",");
    response_string.push_str(&servers_string);
    response_string.push('}');
    HttpResponse::Ok()
        .content_type("application/json")
        .body(response_string)
}

fn main() {
    env::set_var("RUST_LOG", "actix_web=info");
    env::set_var("RUST_BACKTRACE", "1");
    env_logger::init();
    let sys = actix::System::new("server-manager-rust");

    add_server(String::from("8:8:8:8"), String::from("dummy server"));

    server::new(
        || App::new()
            //enable logger
            .middleware(middleware::Logger::default())

            .resource("", |r| r.f(index))
            .resource("/", |r| r.f(index))

            .resource("/json", |r| r.method(Method::POST).f(json_endpoint))

            .default_resource(|r| {
                // 404 for GET request
                r.method(Method::GET).f(p404);
            }))
    .bind("0.0.0.0:8080").expect("Can not bind to 0.0.0.0:8080")
    .shutdown_timeout(0)
    .start();

   ALIVE.lock().unwrap().insert(String::from("alive"), true);

   thread::spawn(|| {
        loop {
            thread::sleep(time::Duration::from_millis(20000));
            clear_servers();

            let alive = ALIVE.lock().unwrap();
            let alive = alive.get(&String::from("alive")).unwrap();
            if !alive {
                break;
            }
        }
    });

    println!("starting server 0.0.0.0:8080");
    let _ = sys.run();
    println!("bye bye now");

    ALIVE.lock().unwrap().insert(String::from("alive"), false);
}
