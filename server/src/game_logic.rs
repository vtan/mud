use crate::{
    event_writer::EventWriter,
    game_state::{GameState, Player, Room},
    id::Id,
    line::{line, span, Line, LineSpan},
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
            line(vec![
                span("Try the "),
                span("look").color("white"),
                span(" and "),
                span("north").color("white"),
                span(" commands."),
            ]),
            span(&format_player_count(state.players.len() + 1)).line(),
        ],
    );
    if let Some(room) = state.rooms.get(&room_id) {
        describe_room(player_id, room, writer, state);
    }

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

        if let ["look"] = words[..] {
            if let Some(room) = state.rooms.get(&player.room_id) {
                describe_room(player_id, room, writer, state);
            }
        } else if let ["who"] = words[..] {
            list_players(player_id, writer, state);
        } else if let Some(exit_room_id) = words.get(0).and_then(|exit| {
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
            lines.push(
                span(&format!(
                    "{} {} here.",
                    and_list(&players),
                    are(players.len())
                ))
                .line(),
            );
        }
    }
    lines.push(if room.exits.is_empty() {
        span("There are no exits here.").line()
    } else {
        span("You can go ")
            .line()
            .extend(and_list_span(
                room.exits
                    .keys()
                    .map(|s| span(s).color("blue"))
                    .collect::<Vec<_>>(),
            ))
            .push(span(" from here."))
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
                to_room
                    .exits
                    .iter()
                    .find(|(_, rid)| **rid == from_room_id)
                    .map_or(
                        span(&format!("{} appears.", &player_name)),
                        |(reverse_exit, _)| {
                            span(&format!("{} arrives from {}.", &player_name, reverse_exit))
                        },
                    )
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
    lines.extend(
        state
            .players
            .values()
            .map(|player| span(&player.name).line()),
    );
    writer.tell_many(player_id, &lines)
}

fn and_list(words: &[String]) -> String {
    match words.len() {
        0 => "".to_string(),
        1 => words[0].clone(),
        2 => format!("{} and {}", words[0], words[1]),
        len => format!("{} and {}", words[0..len - 1].join(", "), words[len - 1]),
    }
}

fn and_list_span(mut words: Vec<LineSpan>) -> Vec<LineSpan> {
    match words.len() {
        0 => words,
        1 => words,
        2 => {
            words.insert(1, span(" and "));
            words
        }
        len => {
            words.insert(len - 1, span(" and "));
            for i in (1..len - 1).rev() {
                words.insert(i, span(", "));
            }
            words
        }
    }
}

fn are(len: usize) -> &'static str {
    if len > 1 {
        "are"
    } else {
        "is"
    }
}

fn plural(len: usize, str: &str) -> String {
    if len > 1 {
        format!("{}s", str)
    } else {
        str.to_string()
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
