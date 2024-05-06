use std::{
    collections::HashMap,
    ops::{Add, Sub},
};

pub use challonge::Challonge;
use challonge::{
    tournament::{
        GamePoints, RankedBy, Tournament, TournamentCreate, TournamentId, TournamentIncludes,
        TournamentState, TournamentType,
    },
    MatchScore, MatchScores,
};
use chrono::*;

pub const SUBDOMAIN: &str = "89c2a59aadab1761b8e29117";
pub const API_KEY: &str = "";

pub fn create_tournament(c: &Challonge, url: String, title: String) -> challonge::Tournament {
    let tc = TournamentCreate {
        name: title.to_owned(),
        tournament_type: TournamentType::SingleElimination,
        url,
        subdomain: SUBDOMAIN.to_string(),
        description: "Test tournament created from challonge-rs".to_owned(),
        open_signup: false,
        hold_third_place_match: false,
        ranked_by: RankedBy::PointsScored,
        show_rounds: false,
        private: false,
        notify_users_when_matches_open: true,
        notify_users_when_the_tournament_ends: true,
        sequential_pairings: false,
        signup_cap: 4,
        start_at: Some(Utc::now().sub(Duration::days(1))),
        check_in_duration: 60,
        grand_finals_modifier: None,
        swiss_points: GamePoints::default(),
        swiss_rounds: 0,
        round_robin_points: GamePoints::default(),
        game_name: Some("mge".to_owned()),
    };

    c.create_tournament(&tc).unwrap()
}

pub fn add_participant(tc: &Tournament, name: &String, steamid: &String) {
    let mut mp = std::collections::HashMap::new();
    mp.insert("api_key", json!(crate::challonge::API_KEY));
    mp.insert(
        "participant",
        json!({"name": name, 
                  "seed": 1,
                  "misc": steamid}),
    );

    let client = reqwest::blocking::Client::new();
    let post = client
        .post(&format!(
            "https://api.challonge.com/v1/tournaments/{}/participants.json",
            tc.id
        ))
        .json(&mp)
        .send()
        .unwrap();

    println!("{:?}", post);
}

pub fn start_tournament(tc: &Tournament) {
    let mut mp = std::collections::HashMap::new();
    mp.insert("api_key", crate::challonge::API_KEY);
    let client = reqwest::blocking::Client::new();
    let put = client
        .post(&format!(
            "https://api.challonge.com/v1/tournaments/{}/start.json",
            tc.id
        ))
        .json(&mp)
        .send()
        .unwrap();
}
use serde_json::{json, Value};

pub fn update_match(tc: &Tournament, m: &Match, winner: &u64, scoreline: &str) {
    let mut mp = std::collections::HashMap::new();
    let mut matches = std::collections::HashMap::new();
    matches.insert("scores_csv", json!(scoreline));
    matches.insert("winner_id", json!(winner));

    mp.insert("api_key", json!(crate::challonge::API_KEY));
    mp.insert("match", json!(matches));

    let client = reqwest::blocking::Client::new();

    println!("reporting match");
    let put = client
        .put(&format!(
            "https://api.challonge.com/v1/tournaments/{}/matches/{:?}.json",
            tc.id, m.id,
        ))
        .json(&mp)
        .send()
        .unwrap();
}

type SteamID = String;

pub fn report_match(c: &Challonge, tc: &Tournament, p1: SteamID, p2: SteamID) {
    let matches = get_matches(&tc.id);
    let participants = c.participant_index(&tc.id).unwrap();
    let pid_to_name: HashMap<u64, (String, SteamID)> = participants
        .0
        .iter()
        .map(|p| (p.id.0, (p.name.clone(), p.misc.clone())))
        .collect::<std::collections::HashMap<_, _>>();

    for m in matches {
        if m.winner_id.is_some() {
            println!("skipping finished match");
            continue;
        }
        match (m.player1_id, m.player2_id) {
            (Some(mp1id), Some(mp2id)) => {
                // if the match has both a player1 and player2
                let mp1 = pid_to_name.get(&mp1id);
                let mp2 = pid_to_name.get(&mp2id);
                match (mp1, mp2) {
                    // (name, steamid), (name, steamid)
                    (Some(mp1), Some(mp2)) => {
                        println!("checking match between {} and {}", mp1.0, mp2.0);
                        if mp1.1 == p1 && mp2.1 == p2 {
                            println!("reporting match between {} and {}", mp1.0, mp2.0);
                            update_match(&tc, &m, &m.player1_id.unwrap(), "1-0");
                        } else if (mp1.1 == p2 && mp2.1 == p1) {
                            println!("reporting match between {} and {}", mp1.0, mp2.0);
                            update_match(&tc, &m, &m.player2_id.unwrap(), "0-1");
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

pub fn pending_matches(
    c: &Challonge,
    tc: &Tournament,
) -> Vec<((String, String), (String, String))> {
    let index = get_matches(&tc.id);

    let participants = c.participant_index(&tc.id).unwrap();

    let pid_to_name: std::collections::HashMap<u64, (String, SteamID)> = participants
        .0
        .iter()
        .map(|p| (p.id.0, (p.name.clone(), p.misc.clone())))
        .collect::<std::collections::HashMap<_, _>>();

    let mut pending_matches = vec![];
    for m in index {
        if m.winner_id.is_some() {
            continue;
        }
        if m.player1_id.is_none() || m.player2_id.is_none() {
            continue;
        }
        let p1 = pid_to_name.get(&m.player1_id.unwrap());
        let p2 = pid_to_name.get(&m.player2_id.unwrap());
        match (p1, p2) {
            (Some(p1), Some(p2)) => {
                pending_matches.push((p1.clone(), p2.clone()));
            }
            _ => {}
        }
    }
    pending_matches
}

pub fn main() {
    let c = Challonge::new("tommylt3", "TUCP3PRoh8aJdYj1Pw5WNT0CJ3kVzCySwaztzM35");
    let cmd = std::env::args().nth(1).unwrap_or("debug".to_string());

    //delete_test_tournaments(&c);
    let mut tc;

    if (cmd == "create") {
        tc = create_tournament(&c, "mge1".to_string(), "weekly tournament 1".to_string());

        let participants = vec![
            ("hallu".to_string(), "76561198000000000".to_string()),
            ("test".to_string(), "76561198000000000".to_string()),
            ("jt".to_string(), "76561198000000000".to_string()),
            ("mang0".to_string(), "76561198000000000".to_string()),
            ("tommy".to_string(), "76561198000000000".to_string()),
            ("cutix".to_string(), "76561198000000000".to_string()),
        ];

        for (name, steamid) in participants {
            //add_participant(&c, &tc, name, steamid);
        }
    } else {
        tc = c
            .get_tournament(
                &TournamentId::Url(SUBDOMAIN.to_string(), "mge1".to_string()),
                &TournamentIncludes::All,
            )
            .unwrap();
    }

    if cmd == "start" {
        c.tournament_start(&tc.id, &TournamentIncludes::All)
            .unwrap()
    }

    let pending = pending_matches(&c, &tc);
    println!("{:?}", pending);

    // println!("Tournament created with id: {}", tc.id);
    println!("Hello, world!");
}

#[derive(serde::Deserialize, Debug)]
struct MatchLike {
    #[serde(rename = "match")]
    pub mat: Match,
}
#[derive(serde::Deserialize, Debug, Clone, Copy)]
#[serde(tag = "match")]
pub struct Match {
    pub id: u64,
    pub player1_id: Option<u64>,
    pub player2_id: Option<u64>,
    pub winner_id: Option<u64>,
}

pub fn get_matches(tid: &challonge_api::TournamentId) -> Vec<Match> {
    let client = reqwest::blocking::Client::new();
    let mut url = reqwest::Url::parse(&format!(
        "https://api.challonge.com/v1/tournaments/{}/matches.json",
        tid.to_string()
    ))
    .unwrap();

    {
        let mut pairs = url.query_pairs_mut();
        pairs.append_pair("api_key", crate::challonge::API_KEY);
        pairs.append_pair("state", "all");
    }

    let index = client.get(url.as_str()).send().unwrap().text();
    let matches: Vec<MatchLike> = serde_json::from_str(&index.unwrap()).unwrap();
    let matches = matches
        .iter()
        .map(|m| m.mat)
        .filter(|m| m.winner_id.is_none())
        .collect::<Vec<_>>();

    matches
}
