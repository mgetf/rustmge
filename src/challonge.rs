use std::ops::{Add, Sub};

pub use challonge::Challonge;
use challonge::ParticipantCreate;
use challonge::{
    tournament::{
        GamePoints, RankedBy, Tournament, TournamentCreate, TournamentId, TournamentIncludes,
        TournamentState, TournamentType,
    },
    MatchScore, MatchScores,
};
use chrono::*;

pub const SUBDOMAIN: &str = "89c2a59aadab1761b8e29117";
pub const API_KEY: &str = "TUCP3PRoh8aJdYj1Pw5WNT0CJ3kVzCySwaztzM35";

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

pub fn update_match(
    tc: &Tournament,
    m: &challonge::Match,
    winner: &challonge::ParticipantId,
    scoreline: &str,
) {
    let mut mp = std::collections::HashMap::new();
    let mut matches = std::collections::HashMap::new();
    matches.insert("scores_csv", json!(scoreline));
    matches.insert("winner_id", json!(winner.0));

    mp.insert("api_key", json!(crate::challonge::API_KEY));
    mp.insert("match", json!(matches));

    let client = reqwest::blocking::Client::new();

    println!("reporting match");
    let put = client
        .put(&format!(
            "https://api.challonge.com/v1/tournaments/{}/matches/{:?}.json",
            tc.id, m.id.0,
        ))
        .json(&mp)
        .send()
        .unwrap();
}

type SteamID = String;

pub fn report_match(c: &Challonge, tc: &Tournament, p1: SteamID, p2: SteamID) {
    let matches = c
        .match_index(&tc.id, Some(challonge::MatchState::All), None)
        .unwrap();
    let participants = c.participant_index(&tc.id).unwrap();
    let pid_to_name = participants
        .0
        .iter()
        .map(|p| (p.id.0, (p.name.clone(), p.misc.clone())))
        .collect::<std::collections::HashMap<_, _>>();

    for m in matches.0 {
        if m.winner_id.is_some() {
            println!("skipping finished match");
            continue;
        }
        let mp1 = pid_to_name.get(&m.player1.id.0);
        let mp2 = pid_to_name.get(&m.player2.id.0);

        match (mp1, mp2) {
            (Some(mp1), Some(mp2)) => {
                println!("checking match between {} and {}", mp1.0, mp2.0);
                if mp1.1 == p1 && mp2.1 == p2 {
                    println!("reporting match between {} and {}", mp1.0, mp2.0);
                    update_match(&tc, &m, &m.player1.id, "1-0");
                } else if (mp1.1 == p2 && mp2.1 == p1) {
                    println!("reporting match between {} and {}", mp1.0, mp2.0);
                    update_match(&tc, &m, &m.player2.id, "0-1");
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
    let index = c
        .match_index(&tc.id, Some(challonge::MatchState::Open), None)
        .unwrap();

    let participants = c.participant_index(&tc.id).unwrap();

    let pid_to_name: std::collections::HashMap<u64, (String, SteamID)> = participants
        .0
        .iter()
        .map(|p| (p.id.0, (p.name.clone(), p.misc.clone())))
        .collect::<std::collections::HashMap<_, _>>();

    index
        .0
        .iter()
        .map(|matc| {
            (
                pid_to_name.get(&matc.player1.id.0).unwrap().clone(),
                pid_to_name.get(&matc.player2.id.0).unwrap().clone(),
            )
        })
        .collect()
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
