use std::ops::Add;

use lazy_static::lazy_static;
use regex::Regex;

use crate::{
    event_writer::EventWriter,
    game_state::{
        Condition, GameState, Player, Room, RoomCommand, RoomDescription, RoomExit, Statement,
    },
    id::Id,
    line::{line, span, Line, LineSpan},
};

lazy_static! {
    static ref HELP_LINES: Vec<Line> = vec![
        span("Commands:").bold().line(),
        span("look").color("white").line().push(span(" – Look around or at something")),
        span("north").color("white").line().push(span(", etc. – Move to another room")),
        span("say").color("white").line().push(span(" – Say something to the others in the room")),
        span("emote").color("white").line().push(span(" – Act out something")),
        span("who").color("white").line().push(span(" – See who is online")),
        span("help").color("white").line().push(span(" – You're looking at it")),
        span("There are also special commands for interacting with specific rooms, or objects in there.").line(),
    ];

    static ref ILLEGAL_CHAT_REGEX: Regex = Regex::new(r"\p{Extended_Pictographic}").unwrap();
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

pub fn on_tick(writer: &mut EventWriter, state: &mut GameState) {
    state.ticks += 1;

    let remaining = state.scheduled_room_var_resets.split_off(&(state.ticks + 1));
    let to_reset = state.scheduled_room_var_resets.clone();
    state.scheduled_room_var_resets = remaining;

    for (room_id, var, message) in to_reset.values() {
        state.set_room_var(*room_id, var.to_string(), 0);
        tell_room(span(message).line(), *room_id, writer, state);
    }
}

enum RoomSpecificCommand<'a> {
    Exit { to_room_id: Id<Room> },
    RoomCommand { room_command: &'a RoomCommand },
}

enum ChatCommand {
    Say,
    Emote,
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
            chat(&player, words, ChatCommand::Say, writer, state);
            Ok(())
        }
        "emote" if !words.is_empty() => {
            chat(&player, words, ChatCommand::Emote, writer, state);
            Ok(())
        }
        "who" if words.is_empty() => {
            list_players(player_id, writer, state);
            Ok(())
        }
        "help" if words.is_empty() => {
            writer.tell_many(player_id, &HELP_LINES);
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

fn resolve_room_specific_command<'a>(
    command: &str,
    args: Vec<&str>,
    room_id: Id<Room>,
    state: &'a GameState,
) -> Result<Option<RoomSpecificCommand<'a>>, String> {
    let room = state.rooms.get(&room_id).ok_or("room specific command: Room not found")?;
    let args_joined = args.join(" ");

    if let Some(to_room_id) = room.exits.get(command).and_then(|exit| match exit {
        RoomExit::Static(to_room_id) => Some(to_room_id),
        RoomExit::Conditional { condition, to } => {
            if eval_room_condition(&condition, room_id, state) {
                Some(to)
            } else {
                None
            }
        }
    }) {
        Ok(Some(RoomSpecificCommand::Exit { to_room_id: *to_room_id }))
    } else if let Some(room_command) = room
        .objects
        .iter()
        .filter(|obj| obj.matches(&args_joined))
        .flat_map(|obj| obj.commands.iter())
        .find(|room_command| {
            if room_command.command != command {
                false
            } else if let Some(cond) = &room_command.condition {
                eval_room_condition(&cond, room_id, state)
            } else {
                true
            }
        })
    {
        Ok(Some(RoomSpecificCommand::RoomCommand { room_command }))
    } else {
        Ok(None)
    }
}

fn eval_room_condition(condition: &Condition, room_id: Id<Room>, state: &GameState) -> bool {
    match condition {
        Condition::Equals(var, value) => state.get_room_var(room_id, var.to_string()) == *value,
        Condition::NotEquals(var, value) => state.get_room_var(room_id, var.to_string()) != *value,
    }
}

fn eval_room_description(
    room_description: &RoomDescription,
    room_id: Id<Room>,
    state: &GameState,
) -> Option<String> {
    match room_description {
        RoomDescription::Static(description) => Some(description.clone()),
        RoomDescription::Dynamic(branches) => {
            let fragments = branches
                .iter()
                .filter_map(|branch| {
                    if branch
                        .condition
                        .as_ref()
                        .map_or(true, |cond| eval_room_condition(cond, room_id, state))
                    {
                        Some(branch.fragment.as_str())
                    } else {
                        None
                    }
                })
                .collect::<Vec<&str>>();
            if fragments.is_empty() {
                None
            } else {
                Some(fragments.join(" "))
            }
        }
    }
}

fn run_room_command(
    room_command: &RoomCommand,
    self_id: Id<Player>,
    room_id: Id<Room>,
    writer: &mut EventWriter,
    state: &mut GameState,
) {
    for statement in &room_command.statements {
        match statement {
            Statement::SetRoomVar(var, value) => {
                state.set_room_var(room_id, var.to_string(), *value);
            }
            Statement::TellSelf(line) => {
                writer.tell(self_id, span(&line).line());
            }
            Statement::TellOthers(line) => {
                let player_name = state.players.get(&self_id).map_or("", |p| &p.name);
                tell_room_except(
                    span(&format!("{} {}", player_name, line)).line(),
                    room_id,
                    self_id,
                    writer,
                    state,
                );
            }
            Statement::TellRoom(line) => {
                tell_room(span(&line).line(), room_id, writer, state);
            }
            Statement::ResetRoomVarAfterTicks(var, delay, message) => {
                state
                    .scheduled_room_var_resets
                    .insert(state.ticks + delay, (room_id, var.clone(), message.clone()));
            }
        }
    }
}

fn describe_room(self_id: Id<Player>, room: &Room, writer: &mut EventWriter, state: &GameState) {
    let mut lines = Vec::new();
    lines.push(span(&room.name).bold().line());
    if let Some(line) = eval_room_description(&room.description, room.id, state) {
        lines.push(span(&line).line());
    }
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

    let visible_exits = room
        .exits
        .iter()
        .filter_map(|(direction, exit)| match exit {
            RoomExit::Static(_) => Some(direction),
            RoomExit::Conditional { condition, .. } => {
                if eval_room_condition(&condition, room.id, state) {
                    Some(direction)
                } else {
                    None
                }
            }
        })
        .map(|direction| span(direction).color("blue"))
        .collect();
    lines.push(if room.exits.is_empty() {
        span("There are no exits here.").line()
    } else {
        span("You can go ")
            .line()
            .extend(and_list_span(visible_exits))
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
        if let Some(object) = room.objects.iter().find(|obj| obj.matches(&target_str)) {
            if let Some(line) = eval_room_description(&object.description, room.id, state) {
                writer.tell(player.id, span(&line).line());
            }
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
            .find(|(_, exit)| match exit {
                RoomExit::Static(to) => from_room_id == *to,
                RoomExit::Conditional { to, .. } => from_room_id == *to,
            })
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

fn chat(
    player: &Player,
    words: Vec<&str>,
    kind: ChatCommand,
    writer: &mut EventWriter,
    state: &GameState,
) {
    let mut words_joined = words.join(" ");
    if words_joined.len() > 128 {
        writer.tell(player.id, span("That message is too long.").line());
    } else if ILLEGAL_CHAT_REGEX.is_match(&words_joined) {
        writer.tell(
            player.id,
            span("That message contains illegal characters.").line(),
        );
    } else {
        match kind {
            ChatCommand::Say => {
                let mut chars = words_joined.chars();
                if let Some(first_char) = chars.next() {
                    words_joined = first_char.to_uppercase().collect::<String>() + chars.as_str()
                }
            }
            _ => (),
        }

        let last_char = words_joined.chars().last().unwrap_or(' ');
        if !last_char.is_ascii_punctuation() {
            words_joined = words_joined.add(".");
        }

        static COLOR: &str = "yellow";
        let to_self = span(&match kind {
            ChatCommand::Say => format!("You say, \"{}\"", &words_joined),
            ChatCommand::Emote => format!("{} {}", &player.name, &words_joined),
        })
        .color(COLOR)
        .line();
        writer.tell(player.id, to_self);

        let to_others = span(&match kind {
            ChatCommand::Say => format!("{} says, \"{}\"", &player.name, &words_joined),
            ChatCommand::Emote => format!("{} {}", &player.name, &words_joined),
        })
        .color(COLOR)
        .line();
        tell_room_except(to_others, player.room_id, player.id, writer, state);
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
