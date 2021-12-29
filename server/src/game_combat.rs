use crate::{
    event_writer::EventWriter,
    game_room::{self, describe_room, RoomTarget},
    game_state::{GameState, IdMap, MobInstance, Player},
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
            let mob_id = mob_instance.id;
            player.attack_target = Some(mob_id);

            let msg_self = format!("You attack the {}.", mob_instance.template.name);
            writer.tell(player_id, span(&msg_self).color("red").line());
            let msg_others = format!(
                "{} attacks the {}.",
                &player.name, mob_instance.template.name
            );
            writer.tell_room_except2(
                span(&msg_others).color("red").line(),
                player.room_id,
                player_id,
                players,
            );

            mob_instances.values_mut().for_each(|mob| {
                if mob.room_id == room.id {
                    mob.hostile_to.insert(player_id);
                }
            });
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

pub fn tick_player_attacks(writer: &mut EventWriter, state: &mut GameState) {
    let GameState { ticks, players, mob_instances, scheduled_mob_spawns, .. } = state;

    let mut killed_mob_ids = Vec::new();
    let mut room_messages = Vec::new();
    players.values_mut().for_each(|player| {
        if player.attack_target.is_none() {
            player.attack_target = mob_instances
                .values()
                .filter(|mob| mob.room_id == player.room_id && mob.hostile_to.contains(&player.id))
                .map(|mob| mob.id)
                .next();
            if let Some(new_target_id) = player.attack_target {
                if let Some(mob) = mob_instances.get(&new_target_id) {
                    let msg_self = format!("You attack the {}.", mob.template.name);
                    writer.tell(player.id, span(&msg_self).color("red").line());
                    let msg_others = format!("{} attacks the {}.", &player.name, mob.template.name);
                    room_messages.push((
                        player.room_id,
                        Some(player.id),
                        span(&msg_others).color("red").line(),
                    ));
                }
            }
        }
        if let Some(target_mob_id) = player.attack_target {
            match mob_instances.get_mut(&target_mob_id) {
                Some(mob) if mob.room_id == player.room_id => {
                    let room_id = player.room_id;
                    let damage = 10;

                    let msg_self =
                        format!("You hit the {} for {} damage.", mob.template.name, damage);
                    writer.tell(player.id, span(&msg_self).color("red").line());
                    let msg_others = format!(
                        "{} hits the {} for {} damage.",
                        player.name, mob.template.name, damage
                    );
                    room_messages.push((
                        room_id,
                        Some(player.id),
                        span(&msg_others).color("red").line(),
                    ));

                    if mob.hp > damage {
                        mob.hp -= 10;
                    } else {
                        let msg = format!("The {} dies.", mob.template.name);
                        room_messages.push((room_id, None, span(&msg).color("red").line()));
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

pub fn tick_mob_attacks(writer: &mut EventWriter, state: &mut GameState) {
    let GameState { players, mob_instances, .. } = state;
    let mut killed_players = vec![];

    mob_instances.values_mut().for_each(|mob| {
        update_mob_target(mob, players, writer);

        if let Some(target) = mob.attack_target.and_then(|id| players.get_mut(&id)) {
            let mob_name = &mob.template.name;
            let damage = mob.template.damage;
            let target_id = target.id;
            let target_name = target.name.clone();

            let killed = damage >= target.hp;
            if killed {
                target.hp = 100;
                target.attack_target = None;
                target.room_id = Id::new(0);
                killed_players.push((target.id, target.room_id));
            } else {
                target.hp -= damage;
            };

            let msg_target = format!("The {} hits you for {} damage.", mob_name, damage);
            writer.tell(target_id, span(&msg_target).color("red").line());
            let msg_others = format!(
                "The {} hits {} for {} damage.",
                mob_name, target_name, damage
            );
            writer.tell_room_except2(
                span(&msg_others).color("red").line(),
                mob.room_id,
                target_id,
                players,
            );

            if killed {
                let msg_target = "You die.";
                writer.tell(target_id, Line::str(msg_target));
                let msg_others = format!("{} dies.", target_name);
                writer.tell_room_except2(Line::str(&msg_others), mob.room_id, target_id, players);
            }
        }
    });
    killed_players.iter().for_each(|(player_id, _)| {
        mob_instances.values_mut().for_each(|mob| {
            if mob.hostile_to.remove(player_id) && mob.attack_target == Some(*player_id) {
                mob.attack_target = None;
            }
        });
    });

    let GameState { rooms, .. } = &*state;
    killed_players.into_iter().for_each(|(player_id, room_id)| {
        if let Some(room) = rooms.get(&room_id) {
            describe_room(player_id, room, writer, state);
        }
    });
}

pub fn update_mob_target(mob: &mut MobInstance, players: &IdMap<Player>, writer: &mut EventWriter) {
    mob.hostile_to = mob
        .hostile_to
        .iter()
        .filter(|player_id| players.contains_key(player_id))
        .copied()
        .collect();

    if let Some(target_id) = mob.attack_target {
        match players.get(&target_id) {
            Some(target) if target.room_id == mob.room_id => (),
            _ => mob.attack_target = None,
        }
    }
    if mob.attack_target.is_none() {
        if let Some(new_target) = mob
            .hostile_to
            .iter()
            .filter_map(|player_id| {
                players.get(player_id).filter(|player| player.room_id == mob.room_id)
            })
            .next()
        {
            mob.attack_target = Some(new_target.id);

            let msg_target = format!("The {} attacks you.", mob.template.name);
            writer.tell(new_target.id, span(&msg_target).color("red").line());
            let msg_others = format!("The {} attacks {}.", mob.template.name, new_target.name);
            writer.tell_room_except2(
                span(&msg_others).color("red").line(),
                mob.room_id,
                new_target.id,
                players,
            );
        }
    }
}
