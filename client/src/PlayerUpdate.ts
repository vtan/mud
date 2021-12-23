export interface PlayerUpdate {
  lines: ReadonlyArray<Line>
}

export interface Line {
  spans: ReadonlyArray<LineSpan>
}

export interface LineSpan {
  text: string,
  bold?: boolean
}
