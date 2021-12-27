use crate::{
    event_writer::EventWriter,
    game_state::{
        Condition, GameState, MobInstance, Player, Room, RoomCommand, RoomDescription, RoomExit,
        RoomObject, Statement,
    },
    id::Id,
    line::span,
    named::Named,
    text_util::{and_span_vecs, and_spans},
};

pub enum RoomTarget<'a> {
    RoomObject { room_object: &'a RoomObject },
    MobInstance { mob_instance: &'a MobInstance },
}

pub fn resolve_target_in_room<'a>(
    target: &str,
    room: &'a Room,
    state: &'a GameState,
) -> Option<RoomTarget<'a>> {
    use RoomTarget::*;

    let mobs = state
        .mob_instances
        .values()
        .filter(|mob_instance| mob_instance.template.matches(target))
        .map(|mob_instance| MobInstance { mob_instance });

    let room_objects = room
        .objects
        .iter()
        .filter(|room_object| room_object.matches(target))
        .map(|room_object| RoomObject { room_object });

    mobs.chain(room_objects).nth(0)
}

pub enum RoomSpecificCommand<'a> {
    Exit { to_room_id: Id<Room> },
    RoomCommand { room_command: &'a RoomCommand },
}

pub fn resolve_room_specific_command<'a>(
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

pub fn eval_room_condition(condition: &Condition, room_id: Id<Room>, state: &GameState) -> bool {
    match condition {
        Condition::Equals(var, value) => state.get_room_var(room_id, var.to_string()) == *value,
        Condition::NotEquals(var, value) => state.get_room_var(room_id, var.to_string()) != *value,
    }
}

pub fn eval_room_description(
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

pub fn run_room_command(
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
                writer.tell_room_except(
                    span(&format!("{} {}", player_name, line)).line(),
                    room_id,
                    self_id,
                    state,
                );
            }
            Statement::TellRoom(line) => {
                writer.tell_room(span(&line).line(), room_id, state);
            }
            Statement::ResetRoomVarAfterTicks(var, delay, message) => {
                state
                    .scheduled_room_var_resets
                    .insert(state.ticks + delay, (room_id, var.clone(), message.clone()));
            }
        }
    }
}

pub fn describe_room(
    self_id: Id<Player>,
    room: &Room,
    writer: &mut EventWriter,
    state: &GameState,
) {
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
            .map(|player| vec![span(&player.name).color("blue")]);
        let mobs = state
            .mob_instances
            .values()
            .filter(|mob| mob.room_id == room.id)
            .map(|mob| vec![span("a "), span(&mob.template.name).color("orange")]);
        let all = players.chain(mobs).collect::<Vec<_>>();
        if !all.is_empty() {
            let line = span("You see ").line().extend(and_span_vecs(all)).push(span(" here."));
            lines.push(line);
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
            .extend(and_spans(visible_exits))
            .push(span(" from here."))
    });

    writer.tell_many(self_id, &lines);
}
