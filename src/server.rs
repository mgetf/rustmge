use crate::{challonge::SUBDOMAIN, ForwardMessage, ServerWs};
use actix::prelude::*;
use actix_web_actors::ws::Message;

const NUM_ARENAS: usize = 16;

pub struct MatchDetails {
    arena_id: String,
    p1_id: String,
    p2_id: String,
}

pub struct Tournament {
    admin: Option<actix::Addr<ServerWs>>,
    servers: Vec<actix::Addr<ServerWs>>,
    players: Vec<crate::Player>,
    arena_to_match: Vec<Option<Vec<String>>>,
    c: Challonge,
    tc: challonge::Tournament,
}

pub fn get_open_arena(arena_to_match: &Vec<Option<Vec<String>>>) -> Option<usize> {
    for (i, match_details) in arena_to_match.iter().enumerate() {
        if match_details.is_none() {
            return Some(i);
        }
    }
    None
}

use challonge::{matches::Player, Challonge};

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
            arena_to_match: vec![None; NUM_ARENAS],
        }
    }

    pub fn send_pending_matches(&mut self) {
        let pending = crate::challonge::pending_matches(&self.c, &self.tc);
        for ((p1, p1id), (p2, p2id)) in pending {
            for server in &self.servers {
                let arena = get_open_arena(&self.arena_to_match).unwrap();
                self.arena_to_match[arena] = Some(vec![p1.clone(), p2.clone()]);

                server.do_send(ForwardMessage {
                    message: crate::MessagePayload::MatchDetails {
                        arenaId: arena as i32,
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
                if self.arena_to_match[arenaId as usize].is_some() {
                    println!("warning! overriding match in arena {:?}", arenaId);
                }
                self.arena_to_match[arenaId as usize] = Some(vec![p1Id.clone(), p2Id.clone()]);

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
            MessagePayload::SetMatchScore {
                arenaId,
                p1Score,
                p2Score,
            } => {
                for server in &self.servers {
                    server.do_send(ForwardMessage {
                        message: MessagePayload::SetMatchScore {
                            arenaId: arenaId.clone(),
                            p1Score: p1Score.clone(),
                            p2Score: p2Score.clone(),
                        },
                        from: msg.from.clone(),
                    });
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
            MessagePayload::TournamentStop {} => {
                for server in &self.servers {
                    server.do_send(ForwardMessage {
                        message: MessagePayload::TournamentStop {},
                        from: msg.from.clone(),
                    });
                }
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
                self.send_pending_matches();
            }
            MessagePayload::MatchBegan { p1Id, p2Id } => {}
            MessagePayload::UsersInServer { players } => {
                println!("recieved players {:?}", players);
                self.players = players;
                for player in &self.players {
                    println!("adding player {:?}", player.name);
                    crate::challonge::add_participant(&self.tc, &player.name, &player.steamId);
                }

                crate::challonge::start_tournament(&self.tc);
                self.send_pending_matches();
            }
            MessagePayload::Error { message } => {
                println!("recieved error {:?}", message);
            }
        }
    }
}
