import { useEffect, useRef, useCallback } from "react"
import type { WsMessage } from "../types/task"

type WsCallback = (msg: WsMessage) => void

const WS_URL = `${location.protocol === "https:" ? "wss:" : "ws:"}//${location.host}/api/v1/events`

export function useTaskWS(onMessage: WsCallback) {
  const wsRef = useRef<WebSocket | null>(null)
  const onMsgRef = useRef(onMessage)
  onMsgRef.current = onMessage

  const connect = useCallback(() => {
    if (wsRef.current?.readyState === WebSocket.OPEN) return

    const ws = new WebSocket(WS_URL)
    ws.onopen = () => console.log("WS connected")
    ws.onmessage = (e) => {
      try {
        const msg: WsMessage = JSON.parse(e.data)
        onMsgRef.current(msg)
      } catch {
        /* ignore */
      }
    }
    ws.onclose = () => {
      wsRef.current = null
      setTimeout(connect, 3000)
    }
    ws.onerror = () => ws.close()
    wsRef.current = ws
  }, [])

  useEffect(() => {
    connect()
    return () => {
      wsRef.current?.close()
      wsRef.current = null
    }
  }, [connect])
}
