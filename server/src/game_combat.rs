use crate::{
    event_writer::EventWriter,
    game_room::{self, RoomTarget},
    game_state::{GameState, Player, Room},
    id::Id,
    line::span,
};

pub fn kill(
    player_id: Id<Player>,
    player_name: &str,
    room_id: Id<Room>,
    args: Vec<&str>,
    writer: &mut EventWriter,
    state: &mut GameState,
) -> Result<(), String> {
    let room = state.rooms.get(&room_id).ok_or("kill: Room not found")?;
    let args_joined = args.join(" ");

    match game_room::resolve_target_in_room(&args_joined, room, state) {
        Some(RoomTarget::MobInstance { mob_instance }) => {
            let id = mob_instance.id;
            let template_id = mob_instance.template.id;
            let respawn_at = state.ticks + 30;

            writer.tell(
                player_id,
                span(&format!("You kill the {}.", mob_instance.template.name)).line(),
            );
            writer.tell_room_except(
                span(&format!(
                    "{} kills the {}.",
                    player_name, mob_instance.template.name
                ))
                .line(),
                room_id,
                player_id,
                state,
            );

            state.mob_instances.remove(&id);
            state.scheduled_mob_spawns.insert(respawn_at, (room_id, template_id));
        }
        Some(_) => {
            writer.tell(player_id, span("You cannot kill that.").line());
        }
        None => {
            writer.tell(player_id, span("You do not see that here.").line());
        }
    }
    Ok(())
}
