import type { WsEvent } from '../types/api';

export function connectTaskEvents(onEvent: (event: WsEvent) => void): WebSocket {
    const wsUrl = new URL('/api/v1/events', window.location.origin);
    wsUrl.protocol = wsUrl.protocol.replace('http', 'ws');

    const socket = new WebSocket(wsUrl.toString());

    socket.addEventListener('message', (event) => {
        try {
            const payload = JSON.parse(event.data) as WsEvent;
            onEvent(payload);
        } catch (error) {
            console.warn('Failed to parse WS event', error);
        }
    });

    return socket;
}
