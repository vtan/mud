import * as React from "react"
import { AppDispatch } from "./AppReducer"
import { connectToServer } from "./ServerConnection"

export interface Props {
  dispatch: AppDispatch
}

export const LoginForm = ({ dispatch }: Props) => {
  const [name, setName] = React.useState("")

  const loginClicked = React.useCallback(e => {
    e.preventDefault()
    const trimmedName = name.trim()
    if (trimmedName !== "") {
      connectToServer(trimmedName, dispatch)
    }
  }, [name])

  return <div className="loginForm">
    <form onSubmit={loginClicked}>
      <div>
        <label>Name:</label>
        <input value={name} onChange={e => setName(e.target.value)} autoFocus />
      </div>
      <button onClick={loginClicked}>Log in</button>
    </form>
  </div>
}
