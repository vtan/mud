use std::collections::HashSet;

use crate::{
    event_writer::EventWriter,
    game_room::{self, describe_room, RoomTarget},
    game_state::{
        player_ids_in_room, player_ids_in_room_except, GameState, IdMap, MobInstance, Player,
    },
    id::Id,
    line::{span, Color, Line},
    tick::TickDuration,
};
use once_cell::sync::Lazy;
use rand::{thread_rng, Rng};

pub static PLAYER_ATTACK_FREQ: Lazy<TickDuration> = Lazy::new(|| TickDuration::from_secs(1.5));
pub static PLAYER_HEAL_FREQ: Lazy<TickDuration> = Lazy::new(|| TickDuration::from_secs(3.0));

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
            writer.tell(player_id, span(&msg_self).color(Color::LightCyan).line());

            let msg_others = format!(
                "{} attacks the {}.",
                &player.name, mob_instance.template.name
            );
            writer.tell_many(
                player_ids_in_room_except(players, room.id, player_id),
                span(&msg_others).color(Color::Cyan).line(),
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

    let players_on_this_tick = players
        .values()
        .filter(|player| ticks.is_on_division(*PLAYER_ATTACK_FREQ, player.attack_offset))
        .map(|player| player.id)
        .collect::<Vec<_>>();

    for player_id in &players_on_this_tick {
        update_player_target(*player_id, mob_instances, players, writer);
    }

    for player_id in &players_on_this_tick {
        let player = players.get(player_id).unwrap_or_else(|| unreachable!());
        if let Some(target_mob_id) = player.attack_target {
            match mob_instances.get_mut(&target_mob_id) {
                Some(mob) if mob.room_id == player.room_id => {
                    let killed = attack_with_player(player, mob, players, writer);
                    if killed {
                        let respawn_at = *ticks + TickDuration::from_secs(30.0);
                        scheduled_mob_spawns.insert(respawn_at, (mob.room_id, mob.template.id));

                        mob_instances.remove(&target_mob_id);
                        killed_mob_ids.push(target_mob_id);
                    }
                }
                _ => {
                    players.entry(*player_id).and_modify(|p| p.attack_target = None);
                }
            }
        }
    }
    players.values_mut().for_each(|player| match player.attack_target {
        Some(target_mob_id) if killed_mob_ids.contains(&target_mob_id) => {
            player.attack_target = None
        }
        _ => (),
    });
}

fn update_player_target(
    player_id: Id<Player>,
    mob_instances: &IdMap<MobInstance>,
    players: &mut IdMap<Player>,
    writer: &mut EventWriter,
) {
    let player = players.get_mut(&player_id).unwrap_or_else(|| unreachable!());
    let room_id = player.room_id;
    if player.attack_target.is_none() {
        let next_target = mob_instances
            .values()
            .find(|mob| mob.room_id == player.room_id && mob.hostile_to.contains(&player.id));

        if let Some(mob) = next_target {
            player.attack_target = Some(mob.id);

            let msg_self = format!("You attack the {}.", mob.template.name);
            writer.tell(player.id, span(&msg_self).color(Color::LightCyan).line());
            let msg_others = format!("{} attacks the {}.", &player.name, mob.template.name);
            writer.tell_many(
                player_ids_in_room_except(players, room_id, player_id),
                span(&msg_others).color(Color::Cyan).line(),
            );
        }
    }
}

pub fn attack_with_player(
    player: &Player,
    mob: &mut MobInstance,
    players: &IdMap<Player>,
    writer: &mut EventWriter,
) -> bool {
    let room_id = player.room_id;
    let damage = 10;

    let msg_self = format!("You hit the {} for {} damage.", mob.template.name, damage);
    writer.tell(player.id, span(&msg_self).color(Color::LightCyan).line());
    let msg_others = format!(
        "{} hits the {} for {} damage.",
        player.name, mob.template.name, damage
    );
    writer.tell_many(
        player_ids_in_room_except(players, room_id, player.id),
        span(&msg_others).color(Color::Cyan).line(),
    );

    let killed = damage >= mob.hp;
    if killed {
        let msg = format!("The {} dies.", mob.template.name);
        writer.tell_many(
            player_ids_in_room(players, room_id),
            span(&msg).color(Color::DarkGrey).line(),
        );
    } else {
        mob.hp -= 10;
    }
    killed
}

pub fn tick_mob_attacks(writer: &mut EventWriter, state: &mut GameState) {
    let GameState { ticks, players, mob_instances, .. } = state;
    let mut killed_players = vec![];

    let mobs_on_this_tick = mob_instances
        .values()
        .filter(|mob| ticks.is_on_division(mob.template.attack_period, mob.attack_offset))
        .map(|mob| mob.id)
        .collect::<Vec<_>>();

    for mob_id in mobs_on_this_tick {
        update_mob_target(
            mob_instances.get_mut(&mob_id).unwrap_or_else(|| unreachable!()),
            players,
            writer,
        );
        let mob = mob_instances.get(&mob_id).unwrap_or_else(|| unreachable!());

        if let Some(target_id) = mob.attack_target {
            let killed = attack_with_mob(mob, target_id, players, writer);
            if killed {
                let respawn_at = Id::new(0);
                killed_players.push((target_id, respawn_at));

                if let Some(player) = players.get_mut(&target_id) {
                    player.room_id = respawn_at;
                }
                mob_instances.values_mut().for_each(|mob| {
                    if mob.hostile_to.remove(&target_id) && mob.attack_target == Some(target_id) {
                        mob.attack_target = None;
                    }
                });
            }
        }
    }

    let GameState { rooms, .. } = &*state;
    killed_players.into_iter().for_each(|(player_id, respawn_room_id)| {
        if let Some(room) = rooms.get(&respawn_room_id) {
            // TODO: tell other players
            describe_room(player_id, room, writer, state);
            writer.room_entities_changed.insert(respawn_room_id);
        }
    });
}

fn update_mob_target(mob: &mut MobInstance, players: &IdMap<Player>, writer: &mut EventWriter) {
    mob.hostile_to.retain(|player_id| players.contains_key(player_id));

    if let Some(target_id) = mob.attack_target {
        match players.get(&target_id) {
            Some(target) if target.room_id == mob.room_id => (),
            _ => mob.attack_target = None,
        }
    }
    if mob.attack_target.is_none() {
        let potential_targets = mob
            .hostile_to
            .iter()
            .filter_map(|player_id| {
                players.get(player_id).filter(|player| player.room_id == mob.room_id)
            })
            .collect::<Vec<_>>();
        let new_target = match potential_targets.len() {
            0 => None,
            len => Some(potential_targets[thread_rng().gen_range(0..len)]),
        };
        if let Some(new_target) = new_target {
            mob.attack_target = Some(new_target.id);

            let msg_target = format!("The {} attacks you.", mob.template.name);
            writer.tell(
                new_target.id,
                span(&msg_target).color(Color::LightRed).line(),
            );
            let msg_others = format!("The {} attacks {}.", mob.template.name, new_target.name);
            writer.tell_many(
                player_ids_in_room_except(players, mob.room_id, new_target.id),
                span(&msg_others).color(Color::Red).line(),
            );
        }
    }
}

fn attack_with_mob(
    mob: &MobInstance,
    target_id: Id<Player>,
    players: &mut IdMap<Player>,
    writer: &mut EventWriter,
) -> bool {
    let mob_name = &mob.template.name;
    let damage = mob.template.damage;

    let target = players.get_mut(&target_id).unwrap_or_else(|| unreachable!());
    let killed = damage >= target.hp;
    if killed {
        target.hp = target.max_hp;
        target.attack_target = None;
    } else {
        target.hp -= damage;
    };
    let target = players.get(&target_id).unwrap_or_else(|| unreachable!());

    let msg_target = format!("The {} hits you for {} damage.", mob_name, damage);
    writer.tell(target_id, span(&msg_target).color(Color::LightRed).line());
    writer.room_entities_changed.insert(mob.room_id);
    let msg_others = format!(
        "The {} hits {} for {} damage.",
        mob_name, target.name, damage
    );
    writer.tell_many(
        player_ids_in_room_except(players, mob.room_id, target_id),
        span(&msg_others).color(Color::Red).line(),
    );

    if killed {
        let msg_target = "You die.";
        writer.tell(target_id, span(msg_target).color(Color::DarkGrey).line());
        let msg_others = format!("{} dies.", target.name);
        writer.tell_many(
            player_ids_in_room_except(players, mob.room_id, target_id),
            span(&msg_others).color(Color::DarkGrey).line(),
        );
    }
    killed
}

pub fn tick_heal_players(writer: &mut EventWriter, state: &mut GameState) {
    if state.ticks.is_on_division(*PLAYER_HEAL_FREQ, TickDuration::zero()) {
        let players_in_combat = state
            .mob_instances
            .values()
            .flat_map(|mob| mob.hostile_to.iter())
            .collect::<HashSet<_>>();

        for player in state.players.values_mut() {
            if player.hp < player.max_hp && !players_in_combat.contains(&player.id) {
                player.hp = (player.hp + player.max_hp / 20).min(player.max_hp);
                writer.room_entities_changed.insert(player.room_id);
            }
        }
    }
}
