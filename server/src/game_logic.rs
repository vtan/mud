use crate::{
    event_writer::EventWriter,
    game_chat::{self, ChatCommand},
    game_help,
    game_room::{
        describe_room, eval_room_description, resolve_room_specific_command, run_room_command,
        RoomSpecificCommand,
    },
    game_state::{GameState, Player, Room},
    id::Id,
    line::{line, span},
    text_util::{are, plural},
};

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

    let remaining = state.scheduled_room_var_resets.split_off(&(state.ticks + 1));
    let to_reset = state.scheduled_room_var_resets.clone();
    state.scheduled_room_var_resets = remaining;

    for (room_id, var, message) in to_reset.values() {
        state.set_room_var(*room_id, var.to_string(), 0);
        writer.tell_room(span(message).line(), *room_id, state);
    }
}

pub fn on_command(
    player_id: Id<Player>,
    command: &str,
    writer: &mut EventWriter,
    state: &mut GameState,
) -> Result<(), String> {
    let mut words: Vec<&str> = command.split_whitespace().collect();
    let command_head = words.get(0).ok_or("Empty command")?.to_ascii_lowercase();
    words.remove(0);
    let words = words;

    let player = state.players.get(&player_id).ok_or("Self player not found")?;

    match command_head.as_str() {
        "look" => look(&player, words, writer, state),
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
        if let Some(object) = room.objects.iter().find(|obj| obj.matches(&target_str)) {
            if let Some(line) = eval_room_description(&object.description, room.id, state) {
                writer.tell(player.id, span(&line).line());
            }
            writer.tell_room_except(
                span(&format!("{} looks at the {}.", &player.name, &object.name)).line(),
                room.id,
                player.id,
                state,
            );
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
