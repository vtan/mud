import * as React from "react";

import { AppDispatch, AppState, StoredLine } from "./AppReducer";
import { EntityInfo } from "./PlayerUpdate";
import { sendCommand } from "./ServerConnection";

export interface Props {
  state: AppState;
  dispatch: AppDispatch;
}

export const GameComponent = ({ state, dispatch }: Props) => {
  const { websocket, lines, roomInfo } = state;

  const refLogContainer = React.useRef<HTMLDivElement>(null);
  React.useEffect(() => {
    if (refLogContainer.current !== null) {
      refLogContainer.current.scrollTop = refLogContainer.current.scrollHeight;
    }
  }, [lines]);

  const [command, setCommand] = React.useState("");
  const onCommandSubmit = React.useCallback(
    (e) => {
      e.preventDefault();
      const trimmed = command.trim();
      if (websocket && trimmed !== "") {
        dispatch({ type: "commandSubmitted", command: trimmed });
        sendCommand(trimmed, websocket);
        setCommand("");
      }
    },
    [websocket, command]
  );
  const onCommandChange = React.useCallback(
    (e) => setCommand(e.target.value),
    []
  );

  return (
    <div className="fullHeight">
      <div className="mainContainer">
        <div ref={refLogContainer} className="lineContainer">
          {lines.map((line) => (
            <LineComponent key={line.id} {...line} />
          ))}
        </div>
        <div className="commandInput">
          <form onSubmit={onCommandSubmit}>
            <input
              onChange={onCommandChange}
              value={command}
              autoFocus
              className="commandFont"
            />
          </form>
        </div>
      </div>
      <div className="sidebar">
        {roomInfo && (
          <div className="roomEntities">
            <RoomEntityRow entity={roomInfo.selfPlayer} nameClass="white" />
            {roomInfo.players.map((player) => (
              <RoomEntityRow
                key={`p-${player.id}`}
                entity={player}
                nameClass="light-cyan"
              />
            ))}
            {roomInfo.mobs.map((mob) => (
              <RoomEntityRow
                key={`m-${mob.id}`}
                entity={mob}
                nameClass="orange"
              />
            ))}
          </div>
        )}
      </div>
    </div>
  );
};

const LineComponent = (props: StoredLine) => {
  const {
    type,
    line: { spans },
  } = props;

  let className = "line" + (type === "input" ? " command" : " event");
  return (
    <div className={className}>
      {type === "input" && "> "}
      {spans.map((span, i) => {
        let className = "";
        if (span.bold === true) {
          className += " bold";
        }
        if (span.color) {
          className += " " + span.color;
        }
        return (
          <span key={i} className={className}>
            {span.text}
          </span>
        );
      })}
    </div>
  );
};

const RoomEntityRow = (props: { entity: EntityInfo; nameClass: string }) => (
  <div>
    <div className={`name ${props.nameClass}`}>{props.entity.name}</div>
    <Gauge filled={props.entity.hp} total={props.entity.maxHp} />
  </div>
);

const Gauge = (props: { filled: number; total: number }) => {
  const { filled, total } = props;
  return (
    <div className="gauge">
      <div className="filled" style={{ width: `${(100 * filled) / total}%` }} />
      <div>
        {filled}/{total}
      </div>
    </div>
  );
};
