use crate::{DomainError, GameRole, PrivateRoleView, RoomState};

/// Returns private role details for one room member.
pub fn private_role_view(
    room: &RoomState,
    player_id: &str,
) -> Result<PrivateRoleView, DomainError> {
    if !room
        .players
        .iter()
        .any(|p| p.id == player_id && p.connected)
    {
        return Err(DomainError::NotMember);
    }
    let category = room.category.clone().ok_or(DomainError::CategoryNotSet)?;
    let round = room.round.as_ref().ok_or(DomainError::NotInProgress)?;
    if round.imposter_player_id == player_id {
        return Ok(PrivateRoleView {
            game_role: GameRole::Imposter,
            category,
            topic_id: None,
        });
    }
    Ok(PrivateRoleView {
        game_role: GameRole::Crew,
        category,
        topic_id: Some(round.topic_id.clone()),
    })
}
