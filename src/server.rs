use crate::{challonge::SUBDOMAIN, ForwardMessage, ServerWs};
use actix::prelude::*;
use actix_web_actors::ws::Message;

pub struct MatchDetails {
    arena_id: String,
    p1_id: String,
    p2_id: String,
}

pub struct Tournament {
    admin: Option<actix::Addr<ServerWs>>,
    servers: Vec<actix::Addr<ServerWs>>,
    players: Vec<crate::Player>,
    matches: Vec<MatchDetails>,
    c: Challonge,
    tc: challonge::Tournament,
}

use challonge::Challonge;

impl Tournament {
    pub fn new(c: Challonge) -> Self {
        let tid = challonge::TournamentId::Url(SUBDOMAIN.to_string(), "mge1".to_string());
        let tc = c
            .get_tournament(&tid, &challonge::TournamentIncludes::All)
            .unwrap();

        Tournament {
            admin: None,
            servers: vec![],
            c,
            tc,
            players: vec![],
            matches: vec![],
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
            MessagePayload::ServerHello { apiKey, .. } => {
                if apiKey == "admin" {
                    self.admin = Some(msg.from);
                } else {
                    self.servers.push(msg.from);
                }
            }
            MessagePayload::MatchDetails { arenaId, p1Id, p2Id } => {
                for servers in &self.servers {
                    servers.do_send(ForwardMessage {
                        message: MessagePayload::MatchDetails {
                            arenaId: arenaId.clone(),
                            p1Id: p1Id.clone(),
                            p2Id: p2Id.clone(),
                        },
                        from: msg.from.clone(),
                    })
                }
            }
            
            MessagePayload::TournamentStart {} => {
                for server in &self.servers {
                    server.do_send(ForwardMessage {
                        message: MessagePayload::TournamentStart {},
                        from: msg.from.clone(),
                    });
                }
            }
            MessagePayload::MatchCancel {
                delinquents,
                arrived,
                arena,
            } => {
            }
            MessagePayload::MatchResults {
                winner,
                loser,
                finished,
            } => {
                todo!()
            }
            MessagePayload::MatchBegan { p1Id, p2Id } => {

            }
            MessagePayload::UsersInServer { players } => {
                println!("recieved players {:?}", players);
                self.players = players;
            }
            MessagePayload::Error { message } => {
                println!("recieved error {:?}", message);
            }
        }
    }
}
