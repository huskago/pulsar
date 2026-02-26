import type { ClientEvent, ServerEvent } from '$lib/types/events';

let ws: WebSocket | null = $state(null);
let connected = $state(false);
let reconnecting = $state(false);
let heartbeatInterval: ReturnType<typeof setInterval> | null = null;

type EventHandler = (event: ServerEvent) => void;
let handlers: EventHandler[] = [];

function connect(token: string) {
	if (ws?.readyState === WebSocket.OPEN) return;

	const socket = new WebSocket('ws://localhost:3001/gateway');

	socket.onopen = () => {
		send({ type: 'Identify', data: { token } });
	};

	socket.onmessage = (raw) => {
		const event: ServerEvent = JSON.parse(raw.data);

		switch (event.type) {
			case 'Hello':
				connected = true;
				reconnecting = false;
				startHeartbeat(event.data.heartbeat_interval);
				break;

			case 'HeartbeatAck':
				break;

			default:
				for (const handler of handlers) {
					handler(event);
				}
		}
	};

	socket.onclose = () => {
		connected = false;
		stopHeartbeat();

		if (!reconnecting) {
			reconnecting = true;
			setTimeout(() => connect(token), 3000);
		}
	};

	socket.onerror = () => {
		socket.close();
	};

	ws = socket;
}

function disconnect() {
	reconnecting = false;
	stopHeartbeat();
	ws?.close();
	ws = null;
	connected = false;
}

function send(event: ClientEvent) {
	if (ws?.readyState === WebSocket.OPEN) {
		ws.send(JSON.stringify(event));
	}
}

function startHeartbeat(intervalMs: number) {
	stopHeartbeat();
	heartbeatInterval = setInterval(() => {
		send({ type: 'Heartbeat' });
	}, intervalMs);
}

function stopHeartbeat() {
	if (heartbeatInterval) {
		clearInterval(heartbeatInterval);
		heartbeatInterval = null;
	}
}

function onEvent(handler: EventHandler): () => void {
	handlers.push(handler);
	return () => {
		handlers = handlers.filter((h) => h !== handler);
	};
}

function sendMessage(channelId: string, content: string) {
	send({
		type: 'SendMessage',
		data: { channel_id: channelId, content }
	});
}

function startTyping(channelId: string) {
	send({
		type: 'StartTyping',
		data: { channel_id: channelId }
	});
}

export const gateway = {
	get connected() {
		return connected;
	},
	get reconnecting() {
		return reconnecting;
	},
	connect,
	disconnect,
	onEvent,
	sendMessage,
	startTyping
};