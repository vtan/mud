use std::collections::HashSet;

use crate::{
    event_writer::EventWriter,
    game_room::{self, describe_room, RoomTarget},
    game_state::GameState,
    id::{Id, IdMap},
    line::{span, Color, Line},
    mob::Mob,
    player::Player,
    player_coll::PlayerColl,
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
    let GameState { players, rooms, mobs, .. } = state;

    let player = players.by_id().get(&player_id).ok_or("kill: Self not found")?;
    let room = rooms.get(&player.room_id).ok_or("kill: Room not found")?;

    let args_joined = args.join(" ");

    match game_room::resolve_target_in_room(&args_joined, room, mobs.by_id()) {
        Some(RoomTarget::Mob { mob }) => {
            let msg_self = format!("You attack the {}.", mob.template.name);
            writer.tell(player_id, span(&msg_self).color(Color::LightCyan).line());

            let msg_others = format!("{} attacks the {}.", &player.name, mob.template.name);
            writer.tell_many(
                players.ids_in_room_except(room.id, player_id),
                span(&msg_others).color(Color::Cyan).line(),
            );

            let mob_id = mob.id;
            players.modify(&player_id, |player| player.attack_target = Some(mob_id));

            let mob_ids_in_room = mobs.by_room_id().get(&room.id).cloned().unwrap_or_default();
            for mob_id in mob_ids_in_room {
                mobs.modify(&mob_id, |mob| {
                    mob.hostile_to.insert(player_id);
                });
            }
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
    let GameState { ticks, players, mobs, scheduled_mob_spawns, .. } = state;

    let mut killed_mob_ids = Vec::new();

    let players_on_this_tick = players
        .by_id()
        .values()
        .filter(|player| ticks.is_on_division(*PLAYER_ATTACK_FREQ, player.attack_offset))
        .map(|player| player.id)
        .collect::<Vec<_>>();

    for player_id in &players_on_this_tick {
        update_player_target(*player_id, mobs.by_id(), players, writer);
    }

    for player_id in &players_on_this_tick {
        let player = players.by_id().get(player_id).unwrap_or_else(|| unreachable!());
        if let Some(target_mob_id) = player.attack_target {
            match mobs.by_id().get(&target_mob_id) {
                Some(mob) if mob.room_id == player.room_id => {
                    let mob_id = mob.id;
                    let mob_room_id = mob.room_id;
                    let mob_template_id = mob.template.id;
                    let killed = mobs.modify(&mob_id, |mob| {
                        attack_with_player(player, mob, players, writer)
                    });
                    if killed {
                        let respawn_at = *ticks + TickDuration::from_secs(30.0);
                        scheduled_mob_spawns.insert(respawn_at, (mob_room_id, mob_template_id));

                        mobs.remove(&target_mob_id);
                        killed_mob_ids.push(target_mob_id);
                    }
                }
                _ => {
                    players.modify(player_id, |p| p.attack_target = None);
                }
            }
        }
    }

    let players_attacking_killed_mobs = players
        .by_id()
        .values()
        .filter_map(|player| match player.attack_target {
            Some(target_mob_id) if killed_mob_ids.contains(&target_mob_id) => Some(player.id),
            _ => None,
        })
        .collect::<Vec<_>>();
    for player_id in players_attacking_killed_mobs {
        players.modify(&player_id, |p| p.attack_target = None);
    }
}

fn update_player_target(
    player_id: Id<Player>,
    mobs: &IdMap<Mob>,
    players: &mut PlayerColl,
    writer: &mut EventWriter,
) {
    let player = players.by_id().get(&player_id).unwrap_or_else(|| unreachable!());
    let room_id = player.room_id;
    if player.attack_target.is_none() {
        let next_target = mobs
            .values()
            .find(|mob| mob.room_id == player.room_id && mob.hostile_to.contains(&player.id));

        if let Some(mob) = next_target {
            let msg_self = format!("You attack the {}.", mob.template.name);
            writer.tell(player.id, span(&msg_self).color(Color::LightCyan).line());
            let msg_others = format!("{} attacks the {}.", &player.name, mob.template.name);
            writer.tell_many(
                players.ids_in_room_except(room_id, player_id),
                span(&msg_others).color(Color::Cyan).line(),
            );

            players.modify(&player_id, |p| p.attack_target = Some(mob.id));
        }
    }
}

pub fn attack_with_player(
    player: &Player,
    mob: &mut Mob,
    players: &PlayerColl,
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
        players.ids_in_room_except(room_id, player.id),
        span(&msg_others).color(Color::Cyan).line(),
    );

    let killed = damage >= mob.hp;
    if killed {
        let msg = format!("The {} dies.", mob.template.name);
        writer.tell_many(
            players.ids_in_room(room_id),
            span(&msg).color(Color::DarkGrey).line(),
        );
    } else {
        mob.hp -= 10;
    }
    killed
}

pub fn tick_mob_attacks(writer: &mut EventWriter, state: &mut GameState) {
    let GameState { ticks, players, mobs, .. } = state;
    let mut killed_players = vec![];

    let mobs_on_this_tick = mobs
        .by_id()
        .values()
        .filter(|mob| ticks.is_on_division(mob.template.attack_period, mob.attack_offset))
        .map(|mob| mob.id)
        .collect::<Vec<_>>();

    for mob_id in mobs_on_this_tick {
        mobs.modify(&mob_id, |mob| {
            update_mob_target(mob, players, writer);
        });
        let mob = mobs.by_id().get(&mob_id).unwrap_or_else(|| unreachable!());

        if let Some(target_id) = mob.attack_target {
            let killed = attack_with_mob(mob, target_id, players, writer);
            if killed {
                let respawn_at = Id::new(0);
                killed_players.push((target_id, respawn_at));

                players.modify(&target_id, |p| p.room_id = respawn_at);

                let mob_ids_in_room =
                    mobs.by_room_id().get(&mob.room_id).cloned().unwrap_or_default();
                for mob_id in mob_ids_in_room {
                    mobs.modify(&mob_id, |mob| {
                        if mob.hostile_to.remove(&target_id) && mob.attack_target == Some(target_id)
                        {
                            mob.attack_target = None;
                        }
                    });
                }
            }
        }
    }

    let GameState { rooms, .. } = &*state;
    killed_players.into_iter().for_each(|(player_id, respawn_room_id)| {
        if let Some(room) = rooms.get(&respawn_room_id) {
            // TODO: tell other players
            describe_room(player_id, room, writer, state);
        }
    });
}

fn update_mob_target(mob: &mut Mob, players: &PlayerColl, writer: &mut EventWriter) {
    mob.hostile_to.retain(|player_id| players.by_id().contains_key(player_id));

    if let Some(target_id) = mob.attack_target {
        match players.by_id().get(&target_id) {
            Some(target) if target.room_id == mob.room_id => (),
            _ => mob.attack_target = None,
        }
    }
    if mob.attack_target.is_none() {
        let potential_targets = mob
            .hostile_to
            .iter()
            .filter_map(|player_id| {
                players.by_id().get(player_id).filter(|player| player.room_id == mob.room_id)
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
                players.ids_in_room_except(mob.room_id, new_target.id),
                span(&msg_others).color(Color::Red).line(),
            );
        }
    }
}

fn attack_with_mob(
    mob: &Mob,
    target_id: Id<Player>,
    players: &mut PlayerColl,
    writer: &mut EventWriter,
) -> bool {
    let mob_name = &mob.template.name;
    let damage = mob.template.damage;

    let (target_name, killed) = players.modify(&target_id, |target| {
        let killed = damage >= target.hp;
        if killed {
            target.hp = target.max_hp;
            target.attack_target = None;
        } else {
            target.hp -= damage;
        }
        (target.name.clone(), killed)
    });

    let msg_target = format!("The {} hits you for {} damage.", mob_name, damage);
    writer.tell(target_id, span(&msg_target).color(Color::LightRed).line());
    let msg_others = format!(
        "The {} hits {} for {} damage.",
        mob_name, target_name, damage
    );
    writer.tell_many(
        players.ids_in_room_except(mob.room_id, target_id),
        span(&msg_others).color(Color::Red).line(),
    );

    if killed {
        let msg_target = "You die.";
        writer.tell(target_id, span(msg_target).color(Color::DarkGrey).line());
        let msg_others = format!("{} dies.", target_name);
        writer.tell_many(
            players.ids_in_room_except(mob.room_id, target_id),
            span(&msg_others).color(Color::DarkGrey).line(),
        );
    }
    killed
}

pub fn tick_heal_players(state: &mut GameState) {
    if state.ticks.is_on_division(*PLAYER_HEAL_FREQ, TickDuration::zero()) {
        let players_in_combat = state
            .mobs
            .by_id()
            .values()
            .flat_map(|mob| mob.hostile_to.iter())
            .collect::<HashSet<_>>();

        let healed_player_ids = state
            .players
            .by_id()
            .values()
            .filter_map(|player| {
                if player.hp < player.max_hp && !players_in_combat.contains(&player.id) {
                    Some(player.id)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        for player_id in healed_player_ids {
            state
                .players
                .modify(&player_id, |p| p.hp = (p.hp + p.max_hp / 20).min(p.max_hp));
        }
    }
}
