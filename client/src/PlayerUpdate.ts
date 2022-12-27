export interface PlayerUpdate {
  lines: ReadonlyArray<Line>;
  selfInfo?: EntityInfo;
}

export interface Line {
  spans: ReadonlyArray<LineSpan>;
}

export interface LineSpan {
  text: string;
  bold?: boolean;
  color?: string;
}

export interface EntityInfo {
  hp: number;
  maxHp: number;
}
