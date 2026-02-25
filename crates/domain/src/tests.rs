use crate::*;

fn setup_room() -> RoomState {
    let mut rng = FixedRng::new(1, 0);
    let catalog = default_catalog();
    let (room, _) = apply(
        None,
        Command::CreateRoom {
            code: "ABCD".to_string(),
            nickname: "Admin".to_string(),
            token_hash: "h1".to_string(),
        },
        &catalog,
        &mut rng,
    )
    .expect("create");
    room
}

#[test]
fn start_game_rejects_when_players_less_than_3() {
    let mut rng = FixedRng::new(0, 0);
    let catalog = default_catalog();
    let room = setup_room();
    let (room, _) = apply(
        Some(&room),
        Command::SetCategory {
            category: "Animals".to_string(),
        },
        &catalog,
        &mut rng,
    )
    .expect("category set");
    let res = apply(Some(&room), Command::StartGame, &catalog, &mut rng);
    assert_eq!(res.unwrap_err(), DomainError::InsufficientPlayers);
}

#[test]
fn join_room_rejects_when_more_than_10() {
    let mut rng = FixedRng::new(0, 0);
    let catalog = default_catalog();
    let mut room = setup_room();
    for idx in 0..9 {
        let (next, _) = apply(
            Some(&room),
            Command::JoinRoom {
                nickname: format!("U{idx}"),
                token_hash: format!("h{idx}"),
            },
            &catalog,
            &mut rng,
        )
        .expect("join under limit");
        room = next;
    }
    assert_eq!(room.players.len(), 10);
    let err = apply(
        Some(&room),
        Command::JoinRoom {
            nickname: "overflow".to_string(),
            token_hash: "h99".to_string(),
        },
        &catalog,
        &mut rng,
    )
    .unwrap_err();
    assert_eq!(err, DomainError::RoomFull);
}

#[test]
fn start_game_selects_exactly_one_imposter_deterministic() {
    let mut rng = FixedRng::new(2, 1);
    let catalog = default_catalog();
    let mut room = setup_room();
    for idx in 0..2 {
        let (next, _) = apply(
            Some(&room),
            Command::JoinRoom {
                nickname: format!("U{idx}"),
                token_hash: format!("hx{idx}"),
            },
            &catalog,
            &mut rng,
        )
        .expect("join");
        room = next;
    }
    let (room, _) = apply(
        Some(&room),
        Command::SetCategory {
            category: "Countries".to_string(),
        },
        &catalog,
        &mut rng,
    )
    .expect("set category");
    let (room, _) = apply(Some(&room), Command::StartGame, &catalog, &mut rng).expect("start");
    let round = room.round.expect("round");
    assert_eq!(round.imposter_player_id, "p3");
    let imposter_count = room
        .players
        .iter()
        .filter(|p| p.id == round.imposter_player_id)
        .count();
    assert_eq!(imposter_count, 1);
}

#[test]
fn next_turn_wraps_correctly() {
    let mut rng = FixedRng::new(0, 0);
    let catalog = default_catalog();
    let mut room = setup_room();
    for idx in 0..2 {
        let (next, _) = apply(
            Some(&room),
            Command::JoinRoom {
                nickname: format!("U{idx}"),
                token_hash: format!("h{idx}"),
            },
            &catalog,
            &mut rng,
        )
        .expect("join");
        room = next;
    }
    let (room, _) = apply(
        Some(&room),
        Command::SetCategory {
            category: "Animals".to_string(),
        },
        &catalog,
        &mut rng,
    )
    .expect("set");
    let (mut room, _) = apply(Some(&room), Command::StartGame, &catalog, &mut rng).expect("start");
    for _ in 0..3 {
        let (next, _) = apply(Some(&room), Command::NextTurn, &catalog, &mut rng).expect("next");
        room = next;
    }
    assert_eq!(
        room.round.expect("round").current_turn_index,
        0,
        "turn index should wrap after player count"
    );
}

#[test]
fn guess_imposter_records_suspicion_without_ending_game() {
    let mut rng = FixedRng::new(1, 0);
    let catalog = default_catalog();
    let mut room = setup_room();
    for idx in 0..2 {
        let (next, _) = apply(
            Some(&room),
            Command::JoinRoom {
                nickname: format!("U{idx}"),
                token_hash: format!("h{idx}"),
            },
            &catalog,
            &mut rng,
        )
        .expect("join");
        room = next;
    }
    let (room, _) = apply(
        Some(&room),
        Command::SetCategory {
            category: "Animals".to_string(),
        },
        &catalog,
        &mut rng,
    )
    .expect("set");
    let (room, _) = apply(Some(&room), Command::StartGame, &catalog, &mut rng).expect("start");
    let (room, _) = apply(
        Some(&room),
        Command::GuessImposter {
            player_id: "p1".to_string(),
            guessed_player_id: "p2".to_string(),
        },
        &catalog,
        &mut rng,
    )
    .expect("guess");
    assert_eq!(room.phase, GamePhase::InProgress);
    assert!(room.result.is_none());
    assert_eq!(room.round.expect("round").suspicions.len(), 1);
}

#[test]
fn restart_game_resets_phase_and_picks_new_topic_and_imposter() {
    let mut rng = FixedRng::new(0, 0);
    let catalog = default_catalog();
    let mut room = setup_room();
    for idx in 0..2 {
        let (next, _) = apply(
            Some(&room),
            Command::JoinRoom {
                nickname: format!("U{idx}"),
                token_hash: format!("h{idx}"),
            },
            &catalog,
            &mut rng,
        )
        .expect("join");
        room = next;
    }
    let (room, _) = apply(
        Some(&room),
        Command::SetCategory {
            category: "Countries".to_string(),
        },
        &catalog,
        &mut rng,
    )
    .expect("set");
    let (room, _) = apply(Some(&room), Command::StartGame, &catalog, &mut rng).expect("start");
    let initial_round = room.round.clone().expect("initial round");
    let (room, _) = apply(
        Some(&room),
        Command::GuessImposter {
            player_id: "p1".to_string(),
            guessed_player_id: "p1".to_string(),
        },
        &catalog,
        &mut rng,
    )
    .expect("guess");
    let (room, _) =
        apply(Some(&room), Command::RevealResult, &catalog, &mut rng).expect("reveal result");
    let mut restart_rng = FixedRng::new(2, 1);
    let (room, _) = apply(
        Some(&room),
        Command::RestartGame,
        &catalog,
        &mut restart_rng,
    )
    .expect("restart");
    let restarted_round = room.round.expect("restarted round");
    assert_eq!(room.phase, GamePhase::InProgress);
    assert!(room.result.is_none());
    assert_ne!(
        initial_round.imposter_player_id,
        restarted_round.imposter_player_id
    );
    assert_ne!(initial_round.topic_id, restarted_round.topic_id);
}

#[test]
fn reveal_result_completes_round_using_suspicion_board() {
    let mut rng = FixedRng::new(1, 0);
    let catalog = default_catalog();
    let mut room = setup_room();
    for idx in 0..2 {
        let (next, _) = apply(
            Some(&room),
            Command::JoinRoom {
                nickname: format!("U{idx}"),
                token_hash: format!("h{idx}"),
            },
            &catalog,
            &mut rng,
        )
        .expect("join");
        room = next;
    }
    let (room, _) = apply(
        Some(&room),
        Command::SetCategory {
            category: "Countries".to_string(),
        },
        &catalog,
        &mut rng,
    )
    .expect("set");
    let (room, _) = apply(Some(&room), Command::StartGame, &catalog, &mut rng).expect("start");
    let (room, _) = apply(
        Some(&room),
        Command::GuessImposter {
            player_id: "p1".to_string(),
            guessed_player_id: "p2".to_string(),
        },
        &catalog,
        &mut rng,
    )
    .expect("guess p1");
    let (room, _) = apply(
        Some(&room),
        Command::GuessImposter {
            player_id: "p3".to_string(),
            guessed_player_id: "p2".to_string(),
        },
        &catalog,
        &mut rng,
    )
    .expect("guess p3");
    let (room, _) = apply(Some(&room), Command::RevealResult, &catalog, &mut rng).expect("reveal");
    assert_eq!(room.phase, GamePhase::Completed);
    let result = room.result.expect("result");
    assert_eq!(result.guessed_player_id.as_deref(), Some("p2"));
    assert_eq!(result.winner, Winner::Crew);
}

#[test]
fn private_role_for_crew_contains_topic_id() {
    let mut rng = FixedRng::new(1, 0);
    let catalog = default_catalog();
    let mut room = setup_room();
    for idx in 0..2 {
        let (next, _) = apply(
            Some(&room),
            Command::JoinRoom {
                nickname: format!("U{idx}"),
                token_hash: format!("h{idx}"),
            },
            &catalog,
            &mut rng,
        )
        .expect("join");
        room = next;
    }
    let (room, _) = apply(
        Some(&room),
        Command::SetCategory {
            category: "Countries".to_string(),
        },
        &catalog,
        &mut rng,
    )
    .expect("set");
    let (room, _) = apply(Some(&room), Command::StartGame, &catalog, &mut rng).expect("start");
    let role = private_role_view(&room, "p1").expect("role for crew");
    assert_eq!(role.game_role, GameRole::Crew);
    assert!(role.topic_id.is_some());
}

#[test]
fn private_role_for_imposter_has_no_topic_id() {
    let mut rng = FixedRng::new(1, 0);
    let catalog = default_catalog();
    let mut room = setup_room();
    for idx in 0..2 {
        let (next, _) = apply(
            Some(&room),
            Command::JoinRoom {
                nickname: format!("U{idx}"),
                token_hash: format!("h{idx}"),
            },
            &catalog,
            &mut rng,
        )
        .expect("join");
        room = next;
    }
    let (room, _) = apply(
        Some(&room),
        Command::SetCategory {
            category: "Countries".to_string(),
        },
        &catalog,
        &mut rng,
    )
    .expect("set");
    let (room, _) = apply(Some(&room), Command::StartGame, &catalog, &mut rng).expect("start");
    let role = private_role_view(&room, "p2").expect("role for imposter");
    assert_eq!(role.game_role, GameRole::Imposter);
    assert!(role.topic_id.is_none());
}
