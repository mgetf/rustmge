use std::ops::{Add, Sub};

use challonge::tournament::{
    GamePoints, RankedBy, Tournament, TournamentCreate, TournamentId, TournamentIncludes,
    TournamentState, TournamentType,
};
use challonge::Challonge;
use challonge::ParticipantCreate;
use chrono::*;

const SUBDOMAIN: &str = "89c2a59aadab1761b8e29117";

pub fn create_tournament(c: &Challonge, url: String, title: String) -> challonge::Tournament {
    let tc = TournamentCreate {
        name: title.to_owned(),
        tournament_type: TournamentType::SingleElimination,
        url: url,
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

    return c.create_tournament(&tc).unwrap();
}

pub fn add_participant(
    c: &Challonge,
    tc: &Tournament,
    name: String,
    steamid: String,
) -> challonge::Participant {
    let pc = ParticipantCreate {
        name: Some(name.clone()),
        challonge_username: None,
        email: (name.clone() + "@mge.tf").to_owned(),
        seed: 1,
        misc: steamid,
    };

    return c.create_participant(&tc.id, &pc).unwrap();
}

pub fn pending_matches(
    c: &Challonge,
    tc: &Tournament,
) -> Vec<((String, String), (String, String))> {
    let index = c
        .match_index(&tc.id, Some(challonge::MatchState::Open), None)
        .unwrap();

    let participants = c.participant_index(&tc.id).unwrap();

    let pid_to_name: std::collections::HashMap<u64, (String, String)> = participants
        .0
        .iter()
        .map(|p| (p.id.0, (p.name.clone(), p.misc.clone())))
        .collect::<std::collections::HashMap<_, _>>();

    let mut pending = Vec::new();
    for matc in index.0.iter() {
        pending.push((
            pid_to_name.get(&matc.player1.id.0).unwrap().clone(),
            pid_to_name.get(&matc.player2.id.0).unwrap().clone(),
        ));
    }

    return pending;
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
            add_participant(&c, &tc, name, steamid);
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
