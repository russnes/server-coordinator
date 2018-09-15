#[macro_use]
extern crate lazy_static;

extern crate actix;
extern crate actix_web;
extern crate env_logger;
use actix_web::http::{header, Method, StatusCode};
use actix_web::middleware::session::{self, RequestSession};
use actix_web::{
    error, fs, middleware, pred, server, App, Error, HttpRequest, HttpResponse, Path, Result
};

use std::sync::Mutex;
use std::collections::HashMap;
use std::{env, io};

lazy_static! {
    static ref SERVERS: Mutex<HashMap<String, String>> = Mutex::new(HashMap::new());
//    let mut servers = HashMap::new();
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
   server_json_string
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
    HttpResponse::Ok()
        .content_type("text/plain")
        .body("nice price")
}

fn main() {
    env::set_var("RUST_LOG", "actix_web=debug");
    env::set_var("RUST_BACKTRACE", "1");
    env_logger::init();
    let sys = actix::System::new("server-manager-rust");

    add_server(String::from("8:8:8:8"), String::from("dummy server"));

    let addr = server::new(
        || App::new()
            //enable logger
            .middleware(middleware::Logger::default())


            //.resource("", |r| r.method(Method::GET).f(index))
            .resource("", |r| r.f(index))
            .resource("/", |r| r.f(index))
            
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
