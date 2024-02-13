use crate::{ForwardMessage, ServerWs};
use actix::prelude::*;
use actix_web_actors::ws::Message;

pub struct Tournament {
    admin: Option<actix::Addr<ServerWs>>,
    servers: Vec<actix::Addr<ServerWs>>,
    players: Vec<crate::Player>,
    tc: bool,
}

impl Tournament {
    pub fn new() -> Self {
        Tournament {
            admin: None,
            servers: vec![],
            tc: false,
            players: vec![],
        }
    }
}

impl Actor for Tournament {
    type Context = Context<Self>;
}

use crate::MessagePayload;

impl Handler<ForwardMessage> for Tournament {
    type Result = ();

    fn handle(&mut self, msg: ForwardMessage, _ctx: &mut Self::Context) {
        match msg.message {
            MessagePayload::ServerHello {
                apiKey,
                serverNum,
                serverHost,
                serverPort,
                stvPort,
            } => {
                if apiKey == "admin" {
                    self.admin = Some(msg.from);
                } else {
                    self.servers.push(msg.from);
                }
            }
            MessagePayload::AdminInstigateMatch {} => {
                // instigate match, send out match details
                for servers in self.servers.iter() {
                    servers.do_send(ForwardMessage {
                        message: MessagePayload::MatchDetails {
                            arenaId: "1".to_string(),
                            p1Id: "1".to_string(),
                            p2Id: "2".to_string(),
                        },
                        from: msg.from.clone(),
                    });
                }
            }
            MessagePayload::MatchDetails {
                arenaId,
                p1Id,
                p2Id,
            } => {
                todo!("tournametn manager should not receive this")
            }
            MessagePayload::UsersInServerRequest {} => {
                for server in self.servers.iter() {
                    server.do_send(ForwardMessage {
                        message: MessagePayload::UsersInServerRequest {},
                        from: msg.from.clone(),
                    });
                }
            }
            MessagePayload::MatchCanecl {
                delinquents,
                arrived,
                arena,
            } => {
                todo!()
            }
            MessagePayload::MatchResults {
                winner,
                loser,
                finished,
            } => {
                todo!()
            }
            MessagePayload::UsersInServer { players } => {
                println!("recieved players {:?}", players);
                self.players = players;
            }
        }
    }
}
