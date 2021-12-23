import * as React from "react"

import { appReducer, initialAppState } from "./AppReducer"
import { GameComponent } from "./GameComponent"
import { LoginForm } from "./LoginForm"

export const App = () => {
  const [state, dispatch] = React.useReducer(appReducer, initialAppState)

  return state.websocket === null
    ? <LoginForm dispatch={dispatch} />
    : <GameComponent state={state} dispatch={dispatch} />
}
