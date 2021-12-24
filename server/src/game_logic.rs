use lazy_static::lazy_static;

use crate::{
    event_writer::EventWriter,
    game_state::{GameState, Player, Room},
    id::Id,
    line::{line, span, Line, LineSpan},
};

lazy_static! {
    static ref HELP_LINES: Vec<Line> = vec![
        span("Commands:").bold().line(),
        span("look").color("white").line().push(span(" – Look around or at something")),
        span("north").color("white").line().push(span(", etc. – Move to another room")),
        span("who").color("white").line().push(span(" – See who is online")),
        span("help").color("white").line().push(span(" – You're looking at it")),
    ];
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
) -> Result<(), String> {
    let mut words: Vec<&str> = command.split_whitespace().collect();
    let command_head = words.get(0).ok_or("Empty command")?.to_ascii_lowercase();
    words.remove(0);
    let words = words;

    let player = state.players.get(&player_id).ok_or("Self player not found")?;

    match command_head.as_str() {
        "look" => look(&player, words, writer, state),
        "who" if words.is_empty() => {
            list_players(player_id, writer, state);
            Ok(())
        }
        "help" if words.is_empty() => {
            writer.tell_many(player_id, &HELP_LINES);
            Ok(())
        }
        potential_exit => {
            if let Some(exit_room_id) = state
                .rooms
                .get(&player.room_id)
                .and_then(|room| room.exits.get(&potential_exit.to_string()).copied())
            {
                move_self(player_id, exit_room_id, potential_exit, writer, state)
            } else {
                writer.tell(player_id, span("Unknown command.").line());
                Ok(())
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
            .map(|player| span(&player.name).color("blue"))
            .collect::<Vec<_>>();
        match players.len() {
            0 => (),
            len => {
                lines.push(line(and_list_span(players)).push(span(&format!(" {} here.", are(len)))))
            }
        }
    }
    lines.push(if room.exits.is_empty() {
        span("There are no exits here.").line()
    } else {
        span("You can go ")
            .line()
            .extend(and_list_span(
                room.exits.keys().map(|s| span(s).color("blue")).collect::<Vec<_>>(),
            ))
            .push(span(" from here."))
    });
    writer.tell_many(self_id, &lines);
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
        tell_room_except(
            span(&format!("{} looks around.", &player.name)).line(),
            room.id,
            player.id,
            writer,
            state,
        );
    } else {
        if words[0].eq_ignore_ascii_case("at") {
            words.remove(0);
        }
        let words = words;

        let target_str = words.join(" ");
        if let Some(object) =
            room.objects.iter().find(|obj| obj.name.eq_ignore_ascii_case(&target_str))
        {
            writer.tell(player.id, span(&object.description).line());
            tell_room_except(
                span(&format!("{} looks at the {}.", &player.name, &object.name)).line(),
                room.id,
                player.id,
                writer,
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
