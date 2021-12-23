import * as React from "react"

import { AppDispatch, AppState, StoredLine } from "./AppReducer"
import { sendCommand } from "./ServerConnection"

export interface Props {
  state: AppState,
  dispatch: AppDispatch
}

export const GameComponent = ({ state, dispatch }: Props) => {
  const { websocket, lines } = state

  const refLogContainer = React.useRef<HTMLDivElement>(null)
  React.useEffect(
    () => {
      if (refLogContainer.current !== null) {
        refLogContainer.current.scrollTop = refLogContainer.current.scrollHeight
      }
    },
    [lines]
  )

  const [command, setCommand] = React.useState("")
  const onCommandSubmit = React.useCallback(
    e => {
      e.preventDefault()
      const trimmed = command.trim()
      if (websocket && trimmed !== "") {
        dispatch({ type: "commandSubmitted", command: trimmed })
        sendCommand(trimmed, websocket)
        setCommand("")
      }
    },
    [websocket, command]
  )
  const onCommandChange = React.useCallback(
    e => setCommand(e.target.value),
    []
  )

  return <div className="mainContainer">
      <div ref={refLogContainer} className="lineContainer">
        { lines.map(line => <LineComponent key={line.id} {...line} />) }
      </div>
      <div className="commandInput">
        <form onSubmit={onCommandSubmit}>
          <input onChange={onCommandChange} value={command} autoFocus className="commandFont" />
        </form>
      </div>
    </div>
}

const LineComponent = (props: StoredLine) => {
  const { type, line: { spans } } = props;

  let className = "line" + (type === "input" ? " command" : " event")
  return <div className={className}>
    { type === "input" && "> " }
    { spans.map((span, i) => {
        let className = "";
        if (span.bold === true) {
          className += " bold";
        }
        return <span key={i} className={className}>{span.text}</span>
    })}
  </div>
}
