use std::collections::HashSet;
use crate::{challonge::SUBDOMAIN, ForwardMessage, ServerWs};
use actix::prelude::*;

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
    arena_to_match: Vec<Option<HashSet<String>>>,
    arena_priority_order: Vec<i32>,
    c: Challonge,
    tc: challonge::Tournament,
}

pub fn get_open_arena(
    arena_to_match: &Vec<Option<HashSet<String>>>,
    arena_priority_order: &Vec<i32>,
) -> Option<usize> {
    for &arena in arena_priority_order {
        if arena_to_match[arena as usize].is_none() {
            return Some(arena as usize);
        }
    }
    None
}

use actix_web_actors::ws::Message;
use challonge::{matches::Player, Challonge};

impl Tournament {
    pub fn new(c: Challonge, name: String) -> Self {
        let tid = challonge::TournamentId::Url(SUBDOMAIN.to_string(), name.to_string());
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
            arena_priority_order: vec![5, 6, 7, 1, 2, 3, 4, 8, 9, 10, 11, 12, 13, 14, 15, 16], //spire
            //arena_priority_order: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16], //blands mid
        }
    }

    pub fn send_pending_matches(&mut self) {
        let pending = crate::challonge::pending_matches(&self.c, &self.tc);
        'outer: for ((p1, p1id), (p2, p2id)) in pending {
            // skip pending matches that are currently getting played
            for arena in &self.arena_to_match {
                if let Some(mtch) = arena {
                    if mtch.contains(&p1id) || mtch.contains(&p2id) {
                        continue 'outer;
                    }
                }
            }
            let arena = get_open_arena(&self.arena_to_match, &self.arena_priority_order).unwrap();

            let mut mtch = HashSet::new();
            mtch.insert(p1id.clone());
            mtch.insert(p2id.clone());
            self.arena_to_match[arena] = Some(mtch);

            for server in &self.servers {
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
        println!("arenas {:?}", self.arena_to_match);
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
                // this is for when we are receiving a match from the web ui, not likely scenario
                if self.arena_to_match[arenaId as usize].is_some() {
                    println!("warning! overriding match in arena {:?}", arenaId);
                }
                let mut mtch = HashSet::new();
                mtch.insert(p1Id.clone());
                mtch.insert(p2Id.clone());
                self.arena_to_match[arenaId as usize] = Some(mtch);

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
                println!("recieved match score {:?}", arenaId);
                println!("servers {:?}", self.servers);
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
                self.arena_to_match = vec![None; NUM_ARENAS];
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
            } => {
                // TODO TODO TODO TODO TOOD TODO TODO TODO TODO TOODO TO DO
                self.arena_to_match[arena as usize] = None;
            }
            MessagePayload::MatchResults {
                winner,
                loser,
                finished,
                arena,
            } => {
                crate::challonge::report_match(&self.c, &self.tc, winner, loser);
                self.arena_to_match[arena as usize] = None;
                self.send_pending_matches();
            }
            MessagePayload::MatchBegan { p1Id, p2Id } => {}
            MessagePayload::UsersInServer { players } => {
                self.players = players;
                self.players.sort_by(|a, b| a.elo.cmp(&b.elo));
                self.players.reverse();
                println!("recieved players {:?}", self.players);

                let mut seed = 0;

                for player in &self.players {
                    seed += 1;
                    println!("adding player {:?}", player.name);
                    crate::challonge::add_participant(&self.tc, &player.name, &player.steamId, seed);
                }

                crate::challonge::start_tournament(&self.tc);
                self.send_pending_matches();
            }
            MessagePayload::Error { message } => {
                println!("recieved error {:?}", message);
            }
            MessagePayload::ScoreNotification { arena, p1Score, p2Score } => {
                println!("recieved score notifiation on arena {:?}, p1Score {:?}, p2Score {:?}", arena, p1Score, p2Score);
            }
        }
    }
}
