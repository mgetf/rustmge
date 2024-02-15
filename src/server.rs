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

use challonge::{Challonge, Participant};

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

    pub fn send_pending_matches(&self) {
        let pending = crate::challonge::pending_matches(&self.c, &self.tc);
        for ((p1, p1id), (p2, p2id)) in pending {
            for server in &self.servers {
                server.do_send(ForwardMessage {
                    message: crate::MessagePayload::MatchDetails {
                        arenaId: 1,
                        p1Id: p1id.clone(),
                        p2Id: p2id.clone(),
                    },
                    from: server.clone(),
                });
            }
        }
    }
}

impl Actor for Tournament {
    type Context = Context<Self>;
}

use crate::MessagePayload;
use reqwest;
use std::{thread, time};

impl StreamHandler<Result<Response<()>, reqwest::Error>> for Tournament {
    fn handle(&mut self, msg: Result<Response<()>, reqwest::Error>, _ctx: &mut Self::Context) {
        match msg {
            Ok(resp) => {
                println!("got response {:?}", resp);
            }
            Err(err) => {
                println!("got error {:?}", err);
            }
        }
    }
}

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
            MessagePayload::MatchDetails {
                arenaId,
                p1Id,
                p2Id,
            } => {
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

                self.send_pending_matches();
            }
            MessagePayload::MatchCancel {
                delinquents,
                arrived,
                arena,
            } => {}
            MessagePayload::MatchResults {
                winner,
                loser,
                finished,
            } => {
                crate::challonge::report_match(&self.c, &self.tc, winner, loser);
            }
            MessagePayload::MatchBegan { p1Id, p2Id } => {}
            MessagePayload::UsersInServer { players } => {
                println!("recieved players {:?}", players);
                self.players = players;
                for player in &self.players {
                    println!("adding player {:?}", player.name);
                    self.c
                        .create_participant(
                            &self.tc.id,
                            &challonge::ParticipantCreate {
                                name: Some(player.name.clone()),
                                challonge_username: None,
                                email: player.name.clone() + "@mge.tf",
                                seed: 1,
                                misc: player.steamId.clone(),
                            },
                        )
                        .err();
                }

                crate::challonge::start_tournament(&self.tc);
            }
            MessagePayload::Error { message } => {
                println!("recieved error {:?}", message);
            }
        }
    }
}
