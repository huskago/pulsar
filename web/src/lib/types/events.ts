import type { Message, UserStatus } from './models';

export type ServerEvent =
	| { type: 'Hello'; data: { heartbeat_interval: number } }
	| { type: 'MessageCreate'; data: Message }
	| { type: 'PresenceUpdate'; data: { user_id: string; status: UserStatus } }
	| { type: 'TypingStart'; data: { channel_id: string; user_id: string } }
	| { type: 'HeartbeatAck' };

export type ClientEvent =
	| { type: 'Identify'; data: { token: string } }
	| { type: 'Heartbeat' }
	| { type: 'SendMessage'; data: { channel_id: string; content: string } }
	| { type: 'StartTyping'; data: { channel_id: string } };
