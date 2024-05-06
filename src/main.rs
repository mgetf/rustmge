extern crate challonge as challonge_api;

use std::{
    collections::{HashMap, HashSet},
    io::Read,
};

use actix::{Actor, AsyncContext, StreamHandler};
use actix_files::{Files, NamedFile};
use actix_web::{
    get, guard::All, post, web, App, Error, HttpRequest, HttpResponse, HttpServer, Responder,
};
use actix_web_actors::ws;
use serde::{Deserialize, Serialize};
mod challonge;
mod server;

#[derive(Debug, Deserialize, Serialize)]
struct Player {
    steamId: String,
    name: String,
}

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
    // sending
    MatchDetails {
        arenaId: i32,
        p1Id: String,
        p2Id: String,
    },
    MatchBegan {
        p1Id: String,
        p2Id: String,
    },
    TournamentStart {},
    TournamentStop {},
    MatchResults {
        winner: String,
        loser: String,
        finished: bool,
        arena: i32,
    },
    MatchCancel {
        delinquents: Vec<String>,
        arrived: String,
        arena: i32,
    },
    UsersInServer {
        players: Vec<Player>,
    },
    Error {
        message: String,
    },
    SetMatchScore {
        arenaId: i32,
        p1Score: i32,
        p2Score: i32,
    },
}

struct AppState {
    app_name: String,
    tournment: actix::Addr<server::Tournament>,
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
                let parsed: Result<MessagePayload, serde_json::Error> = serde_json::from_str(&text);
                match parsed {
                    Ok(p) => {
                        self.addr.do_send(ForwardMessage {
                            message: p,
                            from: ctx.address(),
                        });
                    }
                    Err(e) => {
                        self.addr.do_send(ForwardMessage {
                            message: MessagePayload::Error {
                                message: e.to_string(),
                            },
                            from: ctx.address(),
                        });
                    }
                }
            }
            Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
            _ => (),
        }
    }
}

async fn server_route(
    req: HttpRequest,
    data: web::Data<AppState>,
    stream: web::Payload,
) -> Result<HttpResponse, Error> {
    let t: &actix::Addr<server::Tournament> = &data.tournment;

    let resp = ws::start(ServerWs { addr: t.clone() }, &req, stream);
    println!("server!!! {:?}", resp);
    resp
}

async fn secret() -> impl Responder {
    NamedFile::open_async("./static/admin.html").await.unwrap()
}

async fn index() -> impl Responder {
    NamedFile::open_async("./static/index.html").await.unwrap()
}

use actix_web::rt::task;
use rusqlite::{Connection, Result};
use valve_server_query::Server;

pub fn server_checkup() {
    let conn = Connection::open("mge.db").unwrap();
    conn.execute(
        "CREATE TABLE IF NOT EXISTS servers (
            ip TEXT NOT NULL,
            time INT NOT NULL,
            name TEXT NOT NULL,
            players INT NOT NULL
        )",
        [],
    )
    .unwrap();
    // query for players at current time: select 1, then select all with that time.
    // query for all data: select all < timestamp, then sum in rust
    conn.execute(
        "CREATE TABLE IF NOT EXISTS aliases (
            name TEXT NOT NULL,
            ip TEXT NOT NULL,
            time INT NOT NULL
        )",
        [],
    )
    .unwrap();
    let mut file = std::fs::File::open("mgeserver.txt").unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();
    let lines: Vec<&str> = contents.split("\n").collect();
    let server_ips = lines
        .iter()
        .map(|x| x.split("//").next().unwrap())
        .map(|x| x.trim())
        .collect::<Vec<&str>>();

    let time: u64 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    for ip in server_ips {
        let ip = ip.to_string();
        let server = Server::new(&ip);
        if let Ok(server) = server {
            let info = server.info();
            let players = server.players();

            if let (Ok(info), Ok(players)) = (info, players) {
                conn.execute(
                    "INSERT INTO servers (ip, time, name, players) VALUES (?1, ?2, ?3, ?4)",
                    &[
                        &ip,
                        &time.to_string(),
                        info.name(),
                        &info.player_count().to_string(),
                    ],
                )
                .unwrap();

                for player in players {
                    conn.execute(
                        "INSERT INTO aliases (name, ip, time) VALUES (?1, ?2, ?3)",
                        &[&player.name(), ip.as_str(), &time.to_string()],
                    )
                    .unwrap();
                }
            }
        }
    }
}

#[derive(Debug, Serialize)]
struct ServerData {
    ip: String,
    time: u64,
    name: String,
    players: u32,
}

#[get("/api/servers")]
pub async fn api_servers() -> impl Responder {
    let conn = Connection::open("mge.db").unwrap();
    let mut stmt = conn
        .prepare("SELECT time FROM servers ORDER BY time DESC LIMIT 1")
        .unwrap();
    let mut most_recent_time: u64 = 0;
    let server_iter = stmt.query_map([], |row| Ok(row.get(0)?));
    for t in server_iter.unwrap() {
        let t: Result<u64, _> = t;
        most_recent_time = t.unwrap();
    }

    let mut stmt = conn
        .prepare("SELECT ip, name, players FROM servers WHERE time = ?1")
        .unwrap();
    let servers = stmt
        .query_map([most_recent_time], |row| {
            Ok(ServerData {
                ip: row.get(0)?,
                time: most_recent_time,
                name: row.get(1)?,
                players: row.get(2)?,
            })
        })
        .unwrap();

    let mut servers_ret = vec![];
    for server in servers {
        servers_ret.push(server.unwrap());
    }

    HttpResponse::Ok().json(servers_ret)
}

#[get("/api/players")]
pub async fn api_players() -> impl Responder {
    let conn = Connection::open("mge.db").unwrap();
    let mut stmt = conn
        .prepare("SELECT time FROM servers ORDER BY time DESC LIMIT 1")
        .unwrap();
    let mut most_recent_time: u64 = 0;
    let server_iter = stmt.query_map([], |row| Ok(row.get(0)?));
    for t in server_iter.unwrap() {
        let t: Result<u64, _> = t;
        most_recent_time = t.unwrap();
    }

    let mut stmt = conn
        .prepare("SELECT ip, name, players FROM servers WHERE time = ?1")
        .unwrap();
    let servers = stmt
        .query_map([most_recent_time], |row| {
            Ok(ServerData {
                ip: row.get(0)?,
                time: most_recent_time,
                name: row.get(1)?,
                players: row.get(2)?,
            })
        })
        .unwrap();

    let mut servers_ret = vec![];
    for server in servers {
        servers_ret.push(server.unwrap());
    }

    HttpResponse::Ok().json(servers_ret)
}

use std::thread;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let c = challonge::Challonge::new("tommylt3", crate::challonge::API_KEY);
    let tournament = Tournament::new(c).start();

    thread::spawn(move || {
        let sys = actix::System::new();
        sys.block_on(async {
            let mut interval = actix::clock::interval(std::time::Duration::from_secs(60 * 5));
            loop {
                interval.tick().await;
                println!("Checking servers!");
                server_checkup();
            }
        });
    });
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(AppState {
                app_name: String::from("Actix-web"),
                tournment: tournament.clone(),
            }))
            .route("/tf2serverep", web::get().to(server_route))
            .route("/secret", web::get().to(secret))
            .route("/", web::get().to(index))
            .service(api_servers)
            .service(api_players)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}

// fn main() {
//     let c = challonge_api::Challonge::new("tommylt3", crate::challonge::API_KEY);
//     let tid = challonge_api::TournamentId::Url(
//         crate::challonge::SUBDOMAIN.to_string(),
//         "mge2".to_string(),
//     );

//     crate::challonge::get_matches(&tid);
// }
