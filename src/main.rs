use actix::{Actor, AsyncContext, StreamHandler};
use actix_files::{Files, NamedFile};
use actix_web::{get, post, web, App, Error, HttpRequest, HttpResponse, HttpServer, Responder};
use actix_web_actors::ws;
use serde::{Deserialize, Serialize};

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
}

struct Server {
    apiKey: String,
    address: actix::Addr<ServerWs>,
}

use actix::prelude::*;

// https://github.com/actix/examples/blob/master/websockets/chat/src/server.rs
struct ServerWs {
    servers: Vec<Server>,
    admin: Option<Server>,
}
impl Actor for ServerWs {
    type Context = ws::WebsocketContext<Self>;
}

#[derive(Message)]
#[rtype(result = "()")]
struct ForwardMessage {
    message: String,
}

impl Handler<ForwardMessage> for ServerWs {
    type Result = ();

    fn handle(&mut self, msg: ForwardMessage, ctx: &mut Self::Context) {
        println!("Forwarding message: {}", msg.message);
        ctx.text(msg.message);
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

                match parsed {
                    MessagePayload::ServerHello {
                        apiKey,
                        serverNum,
                        serverHost,
                        serverPort,
                        stvPort,
                    } => {
                        if apiKey == "admin" {
                            self.admin = Some(Server {
                                apiKey: apiKey,
                                address: ctx.address(),
                            });
                        } else {
                            self.servers.push(Server {
                                apiKey: apiKey,
                                address: ctx.address(),
                            });
                        }
                    }
                    MessagePayload::AdminInstigateMatch {} => {
                        assert!(self.admin.as_mut().unwrap().address == ctx.address());

                        for server in &self.servers {
                            let m: MessagePayload = MessagePayload::MatchDetails {
                                arenaId: String::from("1"),
                                p1Id: String::from("2"),
                                p2Id: String::from("3"),
                            };
                            let p = serde_json::to_string(&m).unwrap();
                            server.address.do_send(ForwardMessage { message: p })
                        }
                    }
                    MessagePayload::MatchDetails {
                        arenaId,
                        p1Id,
                        p2Id,
                    } => {
                        todo!();
                    }
                }

                //ctx.text(text)
            }
            Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
            _ => (),
        }
    }
}

async fn server_route(req: HttpRequest, stream: web::Payload) -> Result<HttpResponse, Error> {
    let resp = ws::start(
        ServerWs {
            servers: vec![],
            admin: None,
        },
        &req,
        stream,
    );
    println!("server!!! {:?}", resp);
    resp
}

async fn index() -> impl Responder {
    NamedFile::open_async("./static/index.html").await.unwrap()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .app_data(web::Data::new(AppState {
                app_name: String::from("Actix-web"),
            }))
            .route("/tf2serverep", web::get().to(server_route))
            .route("/", web::get().to(index))
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
