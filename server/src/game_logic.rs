use std::collections::HashSet;

use crate::{
    event_writer::EventWriter,
    game_alias,
    game_chat::{self, ChatCommand},
    game_combat, game_help,
    game_room::{
        describe_room, eval_room_description, resolve_room_specific_command,
        resolve_target_in_room, run_room_command, RoomSpecificCommand, RoomTarget,
    },
    game_state::{GameState, Room},
    id::Id,
    line::{span, Color, Line},
    mob::{Mob, MobTemplate},
    player::Player,
    text_util::{are, plural},
};
use rand::{thread_rng, Rng};

pub fn initialize(state: &mut GameState) {
    let room_ids_templates = state
        .rooms
        .values()
        .flat_map(|room| {
            let mob_templates = &state.mob_templates;
            room.mob_spawns.iter().filter_map(move |spawn| {
                mob_templates
                    .get(&spawn.mob_template_id)
                    .map(|template| (room.id, template.clone()))
            })
        })
        .collect::<Vec<_>>();
    spawn_mobs(room_ids_templates, state);
}

pub fn on_player_connect(player: Player, writer: &mut EventWriter, state: &mut GameState) {
    let Player { id: player_id, room_id, .. } = player;

    writer.tell_lines(
        player_id,
        &[
            Line::str(&format!("Welcome, {}!", &player.name)),
            Line::spans(vec![
                span("Try to "),
                span("look").color(Color::White),
                span(" around, or check the "),
                span("help").color(Color::White),
                span(" to get your bearings."),
            ]),
            Line::str(&format_player_count(state.players.by_id().len() + 1)),
        ],
    );
    if let Some(room) = state.rooms.get(&room_id) {
        describe_room(player_id, room, writer, state);
    }

    writer.tell_many(
        state.players.ids_in_room(room_id),
        Line::str(&format!("{} appears.", &player.name)),
    );

    state.players.insert(player);
}

pub fn on_player_disconnect(
    player_id: Id<Player>,
    writer: &mut EventWriter,
    state: &mut GameState,
) {
    if let Some(player) = state.players.remove(&player_id) {
        writer.tell_many(
            state.players.ids_in_room(player.room_id),
            Line::str(&format!("{} disappears.", player.name)),
        )
    }
}

pub fn on_tick(writer: &mut EventWriter, state: &mut GameState) {
    state.ticks = state.ticks.increase();
    game_combat::tick_player_attacks(writer, state);
    game_combat::tick_mob_attacks(writer, state);
    game_combat::tick_heal_players(state);
    if state.ticks.is_large_tick() {
        on_large_tick(writer, state);
    }
}

fn on_large_tick(writer: &mut EventWriter, state: &mut GameState) {
    {
        let remaining = state.scheduled_room_var_resets.split_off(&(state.ticks.increase()));
        let to_reset = state.scheduled_room_var_resets.clone();
        state.scheduled_room_var_resets = remaining;

        for (room_id, var, message) in to_reset.values() {
            state.set_room_var(*room_id, var.to_string(), 0);
            writer.tell_many(state.players.ids_in_room(*room_id), Line::str(message));
        }
    }
    {
        let remaining = state.scheduled_mob_spawns.split_off(&(state.ticks.increase()));
        let to_respawn = state.scheduled_mob_spawns.clone();
        state.scheduled_mob_spawns = remaining;

        spawn_mobs(
            to_respawn
                .into_values()
                .filter_map(|(room_id, mob_template_id)| {
                    state.mob_templates.get(&mob_template_id).map(|template| {
                        writer.tell_many(
                            state.players.ids_in_room(room_id),
                            Line::str(&format!("A {} appears.", template.name)),
                        );
                        (room_id, template.clone())
                    })
                })
                .collect(),
            state,
        );
    }
}

pub fn on_command(
    player_id: Id<Player>,
    command: &str,
    writer: &mut EventWriter,
    state: &mut GameState,
) -> Result<(), String> {
    let mut words: Vec<&str> = command.split_whitespace().collect();
    let unresolved_head = words.get(0).ok_or("Empty command")?.to_ascii_lowercase();
    words[0] = &unresolved_head;
    words = game_alias::resolve_aliases(words);

    let command_head = words.remove(0);
    let words = words;

    let player = state.players.by_id().get(&player_id).ok_or("Self player not found")?;

    match command_head {
        "look" => look(player, words, writer, state),
        "kill" => game_combat::kill(player.id, words, writer, state),
        "say" if !words.is_empty() => {
            game_chat::chat(player, words, ChatCommand::Say, writer, state);
            Ok(())
        }
        "emote" if !words.is_empty() => {
            game_chat::chat(player, words, ChatCommand::Emote, writer, state);
            Ok(())
        }
        "who" if words.is_empty() => {
            list_players(player_id, writer, state);
            Ok(())
        }
        "help" if words.is_empty() => {
            game_help::help(player_id, writer);
            Ok(())
        }
        "alias" if words.is_empty() => {
            game_alias::alias(player_id, writer);
            Ok(())
        }
        "roll" if words.is_empty() => roll_die(player, writer, state),

        other_command => {
            let room_specific_command =
                resolve_room_specific_command(other_command, words, player.room_id, state)?;
            match room_specific_command {
                Some(RoomSpecificCommand::Exit { to_room_id }) => {
                    move_self(player_id, to_room_id, other_command, writer, state)
                }
                Some(RoomSpecificCommand::RoomCommand { room_command }) => {
                    run_room_command(
                        &room_command.clone(),
                        player_id,
                        player.room_id,
                        writer,
                        state,
                    );
                    Ok(())
                }
                None => {
                    writer.tell(player_id, Line::str("Unknown command."));
                    Ok(())
                }
            }
        }
    }
}

fn look(
    player: &Player,
    mut words: Vec<&str>,
    writer: &mut EventWriter,
    state: &GameState,
) -> Result<(), String> {
    let room = state.rooms.get(&player.room_id).ok_or("look: Room not found")?;

    if words.is_empty() {
        describe_room(player.id, room, writer, state);
        writer.tell_many(
            state.players.ids_in_room_except(room.id, player.id),
            Line::str(&format!("{} looks around.", &player.name)),
        );
    } else {
        if words[0].eq_ignore_ascii_case("at") {
            words.remove(0);
        }
        let words = words;

        let target_str = words.join(" ");
        if let Some(target) = resolve_target_in_room(&target_str, room, state.mobs.by_id()) {
            match target {
                RoomTarget::RoomObject { room_object: obj } => {
                    if let Some(desc) = eval_room_description(&obj.description, room.id, state) {
                        writer.tell(player.id, Line::str(&desc));
                    }
                    writer.tell_many(
                        state.players.ids_in_room_except(room.id, player.id),
                        Line::str(&format!("{} looks at the {}.", &player.name, &obj.name)),
                    );
                }
                RoomTarget::Mob { mob } => {
                    let mob = &mob.template;
                    writer.tell(player.id, Line::str(&mob.description));
                    writer.tell_many(
                        state.players.ids_in_room_except(room.id, player.id),
                        Line::str(&format!("{} looks at the {}.", &player.name, &mob.name)),
                    );
                }
            }
        } else {
            writer.tell(player.id, Line::str("You do not see that here."));
        }
    }
    Ok(())
}

fn move_self(
    player_id: Id<Player>,
    to_room_id: Id<Room>,
    exit: &str,
    writer: &mut EventWriter,
    state: &mut GameState,
) -> Result<(), String> {
    let to_room = state.rooms.get(&to_room_id).ok_or("move: Room not found")?;
    let (from_room_id, player_name) = state.players.modify(&player_id, |player| {
        let from_room_id = player.room_id;
        let player_name = player.name.clone();
        player.room_id = to_room_id;

        if player.attack_target.is_some() {
            player.attack_target = None;
            writer.tell(player_id, Line::str("You flee."));
        }
        (from_room_id, player_name)
    });

    writer.tell_many(
        state.players.ids_in_room(from_room_id),
        Line::str(&format!("{} leaves {}.", &player_name, exit)),
    );
    writer.tell_many(
        state.players.ids_in_room_except(to_room_id, player_id),
        to_room
            .exit_direction_to(from_room_id)
            .map_or(span(&format!("{} appears.", &player_name)), |direction| {
                span(&format!("{} arrives from {}.", &player_name, direction))
            })
            .line(),
    );

    describe_room(player_id, to_room, writer, state);
    Ok(())
}

fn format_player_count(count: usize) -> String {
    format!(
        "There {} {} {} online.",
        are(count),
        count,
        plural(count, "player")
    )
}

fn list_players(player_id: Id<Player>, writer: &mut EventWriter, state: &GameState) {
    let mut lines = vec![Line::str(&format_player_count(state.players.by_id().len()))];
    lines.extend(state.players.by_id().values().map(|player| Line::str(&player.name)));
    writer.tell_lines(player_id, &lines)
}

fn roll_die(player: &Player, writer: &mut EventWriter, state: &GameState) -> Result<(), String> {
    let mut rng = thread_rng();
    let roll: u32 = rng.gen_range(1..=6);
    writer.tell(player.id, Line::str(&format!("You rolled a {}.", roll)));
    writer.tell_many(
        state.players.ids_in_room_except(player.room_id, player.id),
        Line::str(&format!("{} rolled a {}.", &player.name, roll)),
    );
    Ok(())
}

fn spawn_mobs(room_ids_templates: Vec<(Id<Room>, MobTemplate)>, state: &mut GameState) {
    let GameState { mobs, mob_id_source, .. } = state;
    room_ids_templates
        .into_iter()
        .map(|(room_id, template)| {
            let id = mob_id_source.next();
            let hp = template.max_hp;
            let attack_offset = template.attack_period.random_offset(&mut thread_rng());
            Mob {
                id,
                room_id,
                template,
                hp,
                attack_offset,
                hostile_to: HashSet::new(),
                attack_target: None,
            }
        })
        .for_each(|mob| mobs.insert(mob));
}
