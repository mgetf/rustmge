use actix::{Actor, AsyncContext, StreamHandler};
use actix_files::{Files, NamedFile};
use actix_web::{get, post, web, App, Error, HttpRequest, HttpResponse, HttpServer, Responder};
use actix_web_actors::ws;
use serde::{Deserialize, Serialize};
mod server;

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", content = "payload")]
enum MessagePayload {
    // receiving
    ServerHello {
        apiKey: String,
        serverNum: String,
        serverHost: String,
        serverPort: String,
        stvPort: String,
    },
    // from admin panel, start a match between two players
    AdminInstigateMatch {},

    // sending
    MatchDetails {
        arenaId: String,
        p1Id: String,
        p2Id: String,
    },
}

struct AppState {
    app_name: String,
    tournment: actix::Addr<server::Tournament>,
}

#[derive(Debug)]
struct Server {
    apiKey: String,
    address: actix::Addr<ServerWs>,
}

use crate::server::Tournament;
use actix::prelude::*;

// https://github.com/actix/examples/blob/master/websockets/chat/src/server.rs
struct ServerWs {
    addr: Addr<Tournament>,
}
impl Actor for ServerWs {
    type Context = ws::WebsocketContext<Self>;
}

#[derive(Message)]
#[rtype(result = "()")]
struct ForwardMessage {
    message: MessagePayload,
    from: Addr<ServerWs>,
}

impl Handler<ForwardMessage> for ServerWs {
    type Result = ();

    fn handle(&mut self, msg: ForwardMessage, ctx: &mut Self::Context) {
        println!("Forwarding message: {:?}", msg.message);
        let st = serde_json::to_string(&msg.message).unwrap();
        ctx.text(st);
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for ServerWs {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Pong(_)) => println!("Pong received"),
            Ok(ws::Message::Text(text)) => {
                println!("Text received: {}", text);
                let parsed: MessagePayload =
                    serde_json::from_str(&text).expect("unrecognized json");
                self.addr.do_send(ForwardMessage {
                    message: parsed,
                    from: ctx.address(),
                });
            }
            Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
            _ => (),
        }
    }
}

async fn server_route(req: HttpRequest, stream: web::Payload) -> Result<HttpResponse, Error> {
    let t: &actix::Addr<server::Tournament> = &req.app_data::<AppState>().unwrap().tournment;

    let resp = ws::start(ServerWs { addr: t.clone() }, &req, stream);
    println!("server!!! {:?}", resp);
    resp
}

async fn index() -> impl Responder {
    NamedFile::open_async("./static/index.html").await.unwrap()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let torunament = Tournament::new().start();
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(AppState {
                app_name: String::from("Actix-web"),
                tournment: torunament.clone(),
            }))
            .route("/tf2serverep", web::get().to(server_route))
            .route("/", web::get().to(index))
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
