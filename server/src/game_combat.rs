use crate::{
    event_writer::EventWriter,
    game_room::{self, describe_room, RoomTarget},
    game_state::{GameState, IdMap, MobInstance, Player},
    id::Id,
    line::{span, Line},
    message_stash::MessageStash,
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
            let mut messages = MessageStash::new();

            let mob_id = mob_instance.id;
            player.attack_target = Some(mob_id);

            let msg_self = format!("You attack the {}.", mob_instance.template.name);
            messages.tell(player_id, span(&msg_self).color("red").line());
            let msg_others = format!(
                "{} attacks the {}.",
                &player.name, mob_instance.template.name
            );
            messages.tell_room_except(
                player.room_id,
                player_id,
                span(&msg_others).color("red").line(),
            );

            mob_instances.values_mut().for_each(|mob| {
                if mob.room_id == room.id {
                    mob.hostile_to.insert(player_id);
                }
            });

            messages.write_into(writer, players);
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
    let mut messages = MessageStash::new();
    players.values_mut().for_each(|player| {
        if player.attack_target.is_none() {
            update_player_target(player, mob_instances, &mut messages);
        }
        if let Some(target_mob_id) = player.attack_target {
            match mob_instances.get_mut(&target_mob_id) {
                Some(mob) if mob.room_id == player.room_id => {
                    let room_id = player.room_id;
                    let damage = 10;

                    let msg_self =
                        format!("You hit the {} for {} damage.", mob.template.name, damage);
                    messages.tell(player.id, span(&msg_self).color("red").line());
                    let msg_others = format!(
                        "{} hits the {} for {} damage.",
                        player.name, mob.template.name, damage
                    );
                    messages.tell_room_except(
                        room_id,
                        player.id,
                        span(&msg_others).color("red").line(),
                    );

                    if mob.hp > damage {
                        mob.hp -= 10;
                    } else {
                        let msg = format!("The {} dies.", mob.template.name);
                        messages.tell_room(room_id, span(&msg).color("red").line());
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
    messages.write_into(writer, players);
}

pub fn update_player_target(
    player: &mut Player,
    mob_instances: &IdMap<MobInstance>,
    messages: &mut MessageStash,
) {
    let next_target = mob_instances
        .values()
        .find(|mob| mob.room_id == player.room_id && mob.hostile_to.contains(&player.id));

    if let Some(mob) = next_target {
        player.attack_target = Some(mob.id);

        let msg_self = format!("You attack the {}.", mob.template.name);
        messages.tell(player.id, span(&msg_self).color("red").line());
        let msg_others = format!("{} attacks the {}.", &player.name, mob.template.name);
        messages.tell_room_except(
            player.room_id,
            player.id,
            span(&msg_others).color("red").line(),
        );
    }
}

pub fn tick_mob_attacks(writer: &mut EventWriter, state: &mut GameState) {
    let GameState { players, mob_instances, .. } = state;
    let mut messages = MessageStash::new();
    let mut killed_players = vec![];

    mob_instances.values_mut().for_each(|mob| {
        update_mob_target(mob, players, &mut messages);

        if let Some(target) = mob.attack_target.and_then(|id| players.get_mut(&id)) {
            let mob_name = &mob.template.name;
            let damage = mob.template.damage;

            let killed = damage >= target.hp;
            if killed {
                target.hp = 100;
                target.attack_target = None;
                let respawn_at = Id::new(0);
                killed_players.push((target.id, respawn_at));
            } else {
                target.hp -= damage;
            };

            let msg_target = format!("The {} hits you for {} damage.", mob_name, damage);
            messages.tell(target.id, span(&msg_target).color("red").line());
            let msg_others = format!(
                "The {} hits {} for {} damage.",
                mob_name, target.name, damage
            );
            messages.tell_room_except(
                mob.room_id,
                target.id,
                span(&msg_others).color("red").line(),
            );

            if killed {
                let msg_target = "You die.";
                messages.tell(target.id, Line::str(msg_target));
                let msg_others = format!("{} dies.", target.name);
                messages.tell_room_except(mob.room_id, target.id, Line::str(&msg_others));
            }
        }
    });
    killed_players.iter().for_each(|(player_id, respawn_room_id)| {
        if let Some(player) = players.get_mut(player_id) {
            player.room_id = *respawn_room_id;
        }
        mob_instances.values_mut().for_each(|mob| {
            if mob.hostile_to.remove(player_id) && mob.attack_target == Some(*player_id) {
                mob.attack_target = None;
            }
        });
    });
    messages.write_into(writer, players);

    let GameState { rooms, .. } = &*state;
    killed_players.into_iter().for_each(|(player_id, respawn_room_id)| {
        if let Some(room) = rooms.get(&respawn_room_id) {
            describe_room(player_id, room, writer, state);
        }
    });
}

pub fn update_mob_target(
    mob: &mut MobInstance,
    players: &IdMap<Player>,
    messages: &mut MessageStash,
) {
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
            messages.tell(new_target.id, span(&msg_target).color("red").line());
            let msg_others = format!("The {} attacks {}.", mob.template.name, new_target.name);
            messages.tell_room_except(
                mob.room_id,
                new_target.id,
                span(&msg_others).color("red").line(),
            );
        }
    }
}
