use crate::*;
use domain::{DomainError, FixedRng, default_catalog};

fn build_test_service() -> GameService<InMemoryRoomStore> {
    GameService::with_rng_and_catalog(
        InMemoryRoomStore::default(),
        Box::new(FixedRng::new(1, 0)),
        default_catalog(),
    )
}

#[test]
fn non_admin_cannot_set_category() {
    let service = build_test_service();
    let (_, admin_token) = service
        .create_room("ABCD".to_string(), "Host".to_string())
        .expect("create");
    let (_, user_token) = service
        .join_room("ABCD".to_string(), "Alice".to_string())
        .expect("join");
    let err = service
        .set_category(
            "ABCD".to_string(),
            "Animals".to_string(),
            Session::new(user_token),
        )
        .expect_err("should reject non-admin");
    assert_eq!(err, ServiceError::Forbidden);
    let _ = admin_token;
}

#[test]
fn my_role_requires_matching_token_identity() {
    let service = build_test_service();
    let (_, admin_token) = service
        .create_room("ABCD".to_string(), "Host".to_string())
        .expect("create");
    let (_, _) = service
        .join_room("ABCD".to_string(), "Alice".to_string())
        .expect("join");
    let err = service
        .my_role(
            "ABCD".to_string(),
            Session::new("invalid-token".to_string()),
        )
        .expect_err("invalid token");
    assert_eq!(err, ServiceError::InvalidSession);
    let _ = admin_token;
}

#[test]
fn broadcast_emits_on_join() {
    let service = build_test_service();
    let _ = service
        .create_room("ABCD".to_string(), "Host".to_string())
        .expect("create");
    let mut rx = service
        .subscribe_game_updated("ABCD".to_string())
        .expect("subscribe");
    let _ = service
        .join_room("ABCD".to_string(), "Alice".to_string())
        .expect("join");
    let snapshot = rx.try_recv().expect("receive event");
    assert_eq!(snapshot.room.players.len(), 2);
}

#[test]
fn full_flow_service_level() {
    let service = GameService::with_rng_and_catalog(
        InMemoryRoomStore::default(),
        Box::new(FixedRng::new(1, 0)),
        default_catalog(),
    );
    let (_, admin_token) = service
        .create_room("ABCD".to_string(), "Host".to_string())
        .expect("create");
    let (_, p2_token) = service
        .join_room("ABCD".to_string(), "Alice".to_string())
        .expect("join p2");
    let _ = service
        .join_room("ABCD".to_string(), "Bob".to_string())
        .expect("join p3");
    let admin = Session::new(admin_token);
    service
        .set_category("ABCD".to_string(), "Countries".to_string(), admin.clone())
        .expect("set category");
    let started = service
        .start_game("ABCD".to_string(), admin.clone())
        .expect("start");
    assert_eq!(started.room.phase, domain::GamePhase::InProgress);
    let turn = service
        .next_turn("ABCD".to_string(), admin.clone())
        .expect("next");
    assert_eq!(turn.current_turn_index, 1);
    let suspicion = service
        .guess_imposter("ABCD".to_string(), "p2".to_string(), Session::new(p2_token))
        .expect("guess");
    assert_eq!(suspicion.guessed_player_id, "p2");
    let result = service
        .reveal_result("ABCD".to_string(), admin.clone())
        .expect("reveal");
    assert!(result.imposter_player_id.starts_with('p'));
    let restarted = service
        .restart_game("ABCD".to_string(), admin.clone())
        .expect("restart");
    assert_eq!(restarted.room.phase, domain::GamePhase::InProgress);
    let ended = service
        .end_game("ABCD".to_string(), admin)
        .expect("end game");
    assert_eq!(ended.room.phase, domain::GamePhase::Lobby);
}

#[test]
fn leave_room_marks_player_disconnected() {
    let service = build_test_service();
    let _ = service
        .create_room("ABCD".to_string(), "Host".to_string())
        .expect("create");
    let (_, alice_token) = service
        .join_room("ABCD".to_string(), "Alice".to_string())
        .expect("join alice");
    service
        .leave_room("ABCD".to_string(), Session::new(alice_token))
        .expect("leave");
    let snapshot = service.game_snapshot("ABCD".to_string()).expect("snapshot");
    let alice = snapshot
        .room
        .players
        .iter()
        .find(|p| p.nickname == "Alice")
        .expect("alice in snapshot");
    assert!(!alice.connected);
}

#[test]
fn chat_is_rejected_outside_in_progress_phase() {
    let service = build_test_service();
    let (_, admin_token) = service
        .create_room("ABCD".to_string(), "Host".to_string())
        .expect("create");
    let err = service
        .send_chat(
            "ABCD".to_string(),
            "hello".to_string(),
            Session::new(admin_token),
        )
        .expect_err("chat should fail before game starts");
    assert_eq!(err, ServiceError::Domain(DomainError::NotInProgress));
}

#[test]
fn chat_broadcast_emits_message_content() {
    let service = build_test_service();
    let (_, admin_token) = service
        .create_room("ABCD".to_string(), "Host".to_string())
        .expect("create");
    let (_, player_token) = service
        .join_room("ABCD".to_string(), "Alice".to_string())
        .expect("join");
    let _ = service
        .join_room("ABCD".to_string(), "Bob".to_string())
        .expect("join bob");
    let admin = Session::new(admin_token.clone());
    service
        .set_category("ABCD".to_string(), "Countries".to_string(), admin.clone())
        .expect("set category");
    service
        .start_game("ABCD".to_string(), admin)
        .expect("start game");
    service
        .next_turn("ABCD".to_string(), Session::new(admin_token))
        .expect("advance turn to alice");

    let mut rx = service
        .subscribe_chat_messages("ABCD".to_string())
        .expect("subscribe chat");
    let sent = service
        .send_chat(
            "ABCD".to_string(),
            "Where is your clue?".to_string(),
            Session::new(player_token),
        )
        .expect("send chat");
    let received = rx.try_recv().expect("receive chat event");
    assert_eq!(received.sender_nickname, "Alice");
    assert_eq!(received.text, "Where is your clue?");
    assert_eq!(sent.id, received.id);

    let snapshot = service.game_snapshot("ABCD".to_string()).expect("snapshot");
    let turn = snapshot.turn.expect("turn");
    assert_eq!(turn.current_player_id, "p3");
}

#[test]
fn chat_rejected_when_sender_is_not_current_turn_player() {
    let service = build_test_service();
    let (_, admin_token) = service
        .create_room("ABCD".to_string(), "Host".to_string())
        .expect("create");
    let (_, player_token) = service
        .join_room("ABCD".to_string(), "Alice".to_string())
        .expect("join");
    let _ = service
        .join_room("ABCD".to_string(), "Bob".to_string())
        .expect("join bob");
    let admin = Session::new(admin_token);
    service
        .set_category("ABCD".to_string(), "Countries".to_string(), admin.clone())
        .expect("set category");
    service
        .start_game("ABCD".to_string(), admin)
        .expect("start game");
    let err = service
        .send_chat(
            "ABCD".to_string(),
            "off-turn message".to_string(),
            Session::new(player_token),
        )
        .expect_err("chat should fail when not current turn");
    assert_eq!(
        err,
        ServiceError::InvalidInput("chat is allowed only for the current turn player".to_string())
    );
}
