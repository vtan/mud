use crate::{
    event_writer::EventWriter,
    game_room::{self, RoomTarget},
    game_state::{GameState, Player},
    id::Id,
    line::{span, Line},
};

pub fn kill(
    player_id: Id<Player>,
    args: Vec<&str>,
    writer: &mut EventWriter,
    state: &mut GameState,
) -> Result<(), String> {
    let GameState { players, rooms, mob_instances, .. } = state;

    let player = players.get_mut(&player_id).ok_or("kill: Self not found")?;
    let room = rooms.get(&player.room_id).ok_or("kill: Room not found")?;

    let args_joined = args.join(" ");

    match game_room::resolve_target_in_room(&args_joined, room, mob_instances) {
        Some(RoomTarget::MobInstance { mob_instance }) => {
            player.attack_target = Some(mob_instance.id);
            writer.tell(
                player_id,
                span(&format!("You attack the {}.", mob_instance.template.name))
                    .color("red")
                    .line(),
            );
            writer.tell_room_except(
                span(&format!(
                    "{} attacks the {}.",
                    &player.name, mob_instance.template.name
                ))
                .color("red")
                .line(),
                player.room_id,
                player_id,
                state,
            );
        }
        Some(_) => {
            writer.tell(player_id, Line::str("You cannot kill that."));
        }
        None => {
            writer.tell(player_id, Line::str("You do not see that here."));
        }
    }
    Ok(())
}

pub fn deal_player_damage(writer: &mut EventWriter, state: &mut GameState) {
    let GameState { ticks, players, mob_instances, scheduled_mob_spawns, .. } = state;

    let mut killed_mob_ids = Vec::new();
    let mut room_messages = Vec::new();
    players.values_mut().for_each(|player| {
        if let Some(target_mob_id) = player.attack_target {
            match mob_instances.get_mut(&target_mob_id) {
                Some(mob) if mob.room_id == player.room_id => {
                    let room_id = player.room_id;
                    let damage = 10;
                    writer.tell(
                        player.id,
                        span(&format!(
                            "You hit the {} for {} damage.",
                            mob.template.name, damage
                        ))
                        .color("red")
                        .line(),
                    );
                    room_messages.push((
                        room_id,
                        Some(player.id),
                        span(&format!(
                            "{} hits the {} for {} damage.",
                            player.name, mob.template.name, damage
                        ))
                        .color("red")
                        .line(),
                    ));

                    if mob.hp > damage {
                        mob.hp -= 10;
                    } else {
                        room_messages.push((
                            room_id,
                            None,
                            span(&format!("The {} dies.", mob.template.name)).color("red").line(),
                        ));
                        let respawn_at = *ticks + 30;
                        scheduled_mob_spawns.insert(respawn_at, (room_id, mob.template.id));

                        mob_instances.remove(&target_mob_id);
                        killed_mob_ids.push(target_mob_id);
                    }
                }
                _ => {
                    player.attack_target = None;
                }
            }
        }
    });
    players.values_mut().for_each(|player| match player.attack_target {
        Some(target_mob_id) if killed_mob_ids.contains(&target_mob_id) => {
            player.attack_target = None
        }
        _ => (),
    });
    room_messages.into_iter().for_each(|(room_id, except, line)| {
        if let Some(except) = except {
            writer.tell_room_except(line, room_id, except, state);
        } else {
            writer.tell_room(line, room_id, state);
        }
    });
}
