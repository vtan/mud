use crate::{
    event_writer::EventWriter,
    game_state::{GameState, Player, Room},
    id::Id,
    line::{span, Line},
};

pub fn on_player_connect(player: Player, writer: &mut EventWriter, state: &mut GameState) {
    let Player {
        id: player_id,
        room_id,
        ..
    } = player;

    writer.tell_many(
        player_id,
        &[
            span(&format!("Welcome, {}!", &player.name)).line(),
            span("Try the \"look\" and \"north\" commands.").line(),
        ],
    );
    tell_room(
        span(&format!("{} appears.", &player.name)).line(),
        room_id,
        writer,
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
        tell_room(
            span(&format!("{} disappears.", player.name)).line(),
            player.room_id,
            writer,
            state,
        )
    }
}

pub fn on_command(
    player_id: Id<Player>,
    command: &str,
    writer: &mut EventWriter,
    state: &mut GameState,
) {
    if let Some(player) = state.players.get(&player_id) {
        let words: Vec<&str> = command.split_whitespace().collect();

        if let &["look"] = &words[..] {
            if let Some(room) = state.rooms.get(&player.room_id) {
                describe_room(player_id, room, writer, state);
            }
        } else {
            if let Some(exit_room_id) = words.get(0).and_then(|exit| {
                state
                    .rooms
                    .get(&player.room_id)
                    .and_then(|room| room.exits.get(&exit.to_string()).copied())
            }) {
                move_player(player_id, exit_room_id, words[0], writer, state);
            } else {
                writer.tell(player_id, span("Unknown command.").line());
            }
        }
    }
}

fn describe_room(self_id: Id<Player>, room: &Room, writer: &mut EventWriter, state: &GameState) {
    let mut lines = Vec::new();
    lines.push(span(&room.name).bold().line());
    lines.push(span(&room.description).line());
    {
        let players = state
            .players
            .values()
            .filter(|player| player.id != self_id && player.room_id == room.id)
            .map(|player| player.name.clone())
            .collect::<Vec<_>>();
        if !players.is_empty() {
            lines.push(span(&format!("{} {} here.", and_list(&players), are(&players))).line());
        }
    }
    lines.push(if room.exits.is_empty() {
        span("There are no exits here.").line()
    } else {
        span(&format!(
            "You can go {} from here.",
            and_list(&room.exits.keys().cloned().collect::<Vec<_>>())
        ))
        .line()
    });
    writer.tell_many(self_id, &lines);
}

fn move_player(
    player_id: Id<Player>,
    to_room_id: Id<Room>,
    exit: &str,
    writer: &mut EventWriter,
    state: &mut GameState,
) {
    if let Some(to_room) = state.rooms.get(&to_room_id) {
        if let Some(player) = state.players.get_mut(&player_id) {
            let from_room_id = player.room_id;
            let player_name = player.name.clone();
            player.room_id = to_room_id;

            tell_room(
                span(&format!("{} leaves {}.", &player_name, exit)).line(),
                from_room_id,
                writer,
                state,
            );
            tell_room_except(
                (to_room
                    .exits
                    .iter()
                    .find(|(_, rid)| **rid == from_room_id)
                    .map_or(
                        span(&format!("{} appears.", &player_name)),
                        |(reverse_exit, _)| {
                            span(&format!("{} arrives from {}.", &player_name, reverse_exit))
                        },
                    ))
                .line(),
                to_room_id,
                player_id,
                writer,
                state,
            );

            describe_room(player_id, to_room, writer, state);
        }
    }
}

fn and_list(words: &[String]) -> String {
    match words.len() {
        0 => "".to_string(),
        1 => words[0].clone(),
        2 => format!("{} and {}", words[0], words[1]),
        len => format!("{} and {}", words[0..len - 1].join(", "), words[len - 1]),
    }
}

fn are<T>(words: &[T]) -> &str {
    if words.len() > 1 {
        "are"
    } else {
        "is"
    }
}

fn tell_room(line: Line, room_id: Id<Room>, writer: &mut EventWriter, state: &GameState) {
    state.players.values().for_each(|player| {
        if player.room_id == room_id {
            writer.tell(player.id, line.clone());
        }
    })
}

fn tell_room_except(
    line: Line,
    room_id: Id<Room>,
    except: Id<Player>,
    writer: &mut EventWriter,
    state: &GameState,
) {
    state.players.values().for_each(|player| {
        if player.id != except && player.room_id == room_id {
            writer.tell(player.id, line.clone());
        }
    })
}
