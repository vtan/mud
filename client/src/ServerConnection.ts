import { AppDispatch } from "./AppReducer"

export function connectToServer(playerName: string, dispatch: AppDispatch): void {
  const schema = window.location.hostname === "localhost" ? "ws" : "wss"
  const ws = new WebSocket(`${schema}://${window.location.host}/api/ws?name=${decodeURIComponent(playerName)}`)
  ws.onopen = () => dispatch({ type: "websocketConnected", websocket: ws })
  ws.onmessage = (e) => dispatch({ type: "websocketMessage", message: e.data })
  ws.onclose = (e) => dispatch({ type: "websocketClosed", event: e, isError: false })
  ws.onerror = (e) => dispatch({ type: "websocketClosed", event: e, isError: true })
}

export function sendCommand(command: string, ws: WebSocket): void {
  ws.send(command)
}
