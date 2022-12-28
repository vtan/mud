export interface PlayerUpdate {
  lines: ReadonlyArray<Line>;
  roomInfo?: RoomInfo;
}

export interface Line {
  spans: ReadonlyArray<LineSpan>;
}

export interface LineSpan {
  text: string;
  bold?: boolean;
  color?: string;
}

export interface RoomInfo {
  selfPlayer: EntityInfo;
  players: EntityInfo[];
  mobs: EntityInfo[];
}

export interface EntityInfo {
  id: string;
  name: string;
  hp: number;
  maxHp: number;
}
