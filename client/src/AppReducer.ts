import { Line, PlayerUpdate, RoomInfo } from "./PlayerUpdate";

export interface AppState {
  websocket: WebSocket | null;
  lines: ReadonlyArray<StoredLine>;
  nextLineId: number;
  roomInfo: RoomInfo | null;
}

export interface StoredLine {
  type: "output" | "input";
  id: number;
  line: Line;
}

export const initialAppState: AppState = {
  websocket: null,
  lines: [],
  nextLineId: 0,
  roomInfo: null,
};

export type AppAction =
  | { type: "websocketConnected"; websocket: WebSocket }
  | { type: "websocketClosed"; event: Event; isError: boolean }
  | { type: "websocketMessage"; message: string }
  | { type: "commandSubmitted"; command: string };

export type AppDispatch = (_: AppAction) => void;

const maxOutputBlocks = 1000;

export function appReducer(state: AppState, action: AppAction): AppState {
  switch (action.type) {
    case "websocketConnected":
      return { ...state, websocket: action.websocket };

    case "websocketClosed":
      console.warn("Websocket closed");
      return initialAppState;

    case "websocketMessage": {
      try {
        const update: PlayerUpdate = JSON.parse(action.message);
        return applyPlayerUpdate(state, update);
      } catch (ex) {
        console.warn("Failed to parse message on websocket", ex);
        return state;
      }
    }

    case "commandSubmitted":
      return addLine(state, {
        type: "input",
        line: { spans: [{ text: action.command }] },
      });
  }
}

function applyPlayerUpdate(
  state: AppState,
  playerUpdate: PlayerUpdate
): AppState {
  state = playerUpdate.lines.reduce(
    (acc, line) => addLine(acc, { type: "output", line }),
    state
  );
  if (playerUpdate.roomInfo) {
    state = { ...state, roomInfo: playerUpdate.roomInfo };
  }
  return state;
}

function addLine(state: AppState, newLine: Omit<StoredLine, "id">): AppState {
  const line = { ...newLine, id: state.nextLineId };
  const nextOutputBlockId = (state.nextLineId + 1) & 0xffffffff;
  const lines = [...state.lines, line].slice(-maxOutputBlocks);
  return { ...state, nextLineId: nextOutputBlockId, lines };
}
