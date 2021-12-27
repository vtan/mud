use crate::{
    event_writer::EventWriter,
    game_alias,
    game_chat::{self, ChatCommand},
    game_combat, game_help,
    game_room::{
        describe_room, eval_room_description, resolve_room_specific_command,
        resolve_target_in_room, run_room_command, RoomSpecificCommand, RoomTarget,
    },
    game_state::{GameState, MobInstance, MobTemplate, Player, Room},
    id::Id,
    line::{line, span},
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

    writer.tell_many(
        player_id,
        &[
            span(&format!("Welcome, {}!", &player.name)).line(),
            line(vec![
                span("Try to "),
                span("look").color("white"),
                span(" around, or check the "),
                span("help").color("white"),
                span(" to get your bearings."),
            ]),
            span(&format_player_count(state.players.len() + 1)).line(),
        ],
    );
    if let Some(room) = state.rooms.get(&room_id) {
        describe_room(player_id, room, writer, state);
    }

    writer.tell_room(
        span(&format!("{} appears.", &player.name)).line(),
        room_id,
        state,
    );

    state.players.insert(player_id, player);
}

pub fn on_player_disconnect(
    player_id: Id<Player>,
    writer: &mut EventWriter,
    state: &mut GameState,
) {
    if let Some(player) = state.players.remove(&player_id) {
        writer.tell_room(
            span(&format!("{} disappears.", player.name)).line(),
            player.room_id,
            state,
        )
    }
}

pub fn on_tick(writer: &mut EventWriter, state: &mut GameState) {
    state.ticks += 1;
    {
        let remaining = state.scheduled_room_var_resets.split_off(&(state.ticks + 1));
        let to_reset = state.scheduled_room_var_resets.clone();
        state.scheduled_room_var_resets = remaining;

        for (room_id, var, message) in to_reset.values() {
            state.set_room_var(*room_id, var.to_string(), 0);
            writer.tell_room(span(message).line(), *room_id, state);
        }
    }
    {
        let remaining = state.scheduled_mob_spawns.split_off(&(state.ticks + 1));
        let to_respawn = state.scheduled_mob_spawns.clone();
        state.scheduled_mob_spawns = remaining;

        spawn_mobs(
            to_respawn
                .into_values()
                .filter_map(|(room_id, mob_template_id)| {
                    state.mob_templates.get(&mob_template_id).map(|template| {
                        writer.tell_room(
                            span(&format!("A {} appears.", template.name)).line(),
                            room_id,
                            state,
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
    let mut words: Vec<&str> = game_alias::resolve_aliases(command.split_whitespace().collect());
    let command_head = words.get(0).ok_or("Empty command")?.to_ascii_lowercase();
    words.remove(0);
    let words = words;

    let player = state.players.get(&player_id).ok_or("Self player not found")?;

    match command_head.as_str() {
        "look" => look(&player, words, writer, state),
        "kill" => {
            let Player { id, name, room_id } = player;
            game_combat::kill(*id, &name.clone(), *room_id, words, writer, state)
        }
        "say" if !words.is_empty() => {
            game_chat::chat(&player, words, ChatCommand::Say, writer, state);
            Ok(())
        }
        "emote" if !words.is_empty() => {
            game_chat::chat(&player, words, ChatCommand::Emote, writer, state);
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
        "roll" if words.is_empty() => roll_die(&player, writer, state),

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
                    writer.tell(player_id, span("Unknown command.").line());
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
        writer.tell_room_except(
            span(&format!("{} looks around.", &player.name)).line(),
            room.id,
            player.id,
            state,
        );
    } else {
        if words[0].eq_ignore_ascii_case("at") {
            words.remove(0);
        }
        let words = words;

        let target_str = words.join(" ");
        if let Some(target) = resolve_target_in_room(&target_str, room, state) {
            match target {
                RoomTarget::RoomObject { room_object: obj } => {
                    if let Some(desc) = eval_room_description(&obj.description, room.id, state) {
                        writer.tell(player.id, span(&desc).line());
                    }
                    writer.tell_room_except(
                        span(&format!("{} looks at the {}.", &player.name, &obj.name)).line(),
                        room.id,
                        player.id,
                        state,
                    );
                }
                RoomTarget::MobInstance { mob_instance } => {
                    let mob = &mob_instance.template;
                    writer.tell(player.id, span(&mob.description).line());
                    writer.tell_room_except(
                        span(&format!("{} looks at the {}.", &player.name, &mob.name)).line(),
                        room.id,
                        player.id,
                        state,
                    );
                }
            }
        } else {
            writer.tell(player.id, span("You do not see that here.").line());
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
    let mut player = state.players.get_mut(&player_id).ok_or("move: Self player not found")?;

    let from_room_id = player.room_id;
    let player_name = player.name.clone();
    player.room_id = to_room_id;

    writer.tell_room(
        span(&format!("{} leaves {}.", &player_name, exit)).line(),
        from_room_id,
        state,
    );
    writer.tell_room_except(
        to_room
            .exit_direction_to(from_room_id)
            .map_or(span(&format!("{} appears.", &player_name)), |direction| {
                span(&format!("{} arrives from {}.", &player_name, direction))
            })
            .line(),
        to_room_id,
        player_id,
        state,
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
    let mut lines = vec![span(&format_player_count(state.players.len())).line()];
    lines.extend(state.players.values().map(|player| span(&player.name).line()));
    writer.tell_many(player_id, &lines)
}

fn roll_die(player: &Player, writer: &mut EventWriter, state: &GameState) -> Result<(), String> {
    let mut rng = thread_rng();
    let roll: u32 = rng.gen_range(1..=6);
    writer.tell(
        player.id,
        span(&format!("You rolled a {}.", roll.to_string())).line(),
    );
    writer.tell_room_except(
        span(&format!("{} rolled a {}.", &player.name, roll.to_string())).line(),
        player.room_id,
        player.id,
        state,
    );
    Ok(())
}

fn spawn_mobs(room_ids_templates: Vec<(Id<Room>, MobTemplate)>, state: &mut GameState) {
    let GameState { mob_instances, mob_instance_id_source, .. } = state;
    mob_instances.extend(room_ids_templates.into_iter().map(|(room_id, template)| {
        let id = mob_instance_id_source.next();
        let instance = MobInstance { id, room_id, template };
        (id, instance)
    }));
}
