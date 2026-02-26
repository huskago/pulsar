export interface User {
	id: string;
	username: string;
	email: string;
	avatar_url: string | null;
	status: UserStatus;
}

export type UserStatus = 'online' | 'offline' | 'idle' | 'do_not_disturb';

export interface Guild {
	id: string;
	name: string;
	icon_url: string | null;
	owner_id: string;
}

export interface Channel {
	id: string;
	guild_id: string;
	name: string;
	kind: ChannelKind;
	position: number;
}

export type ChannelKind = 'text' | 'voice' | 'category';

export interface Message {
	id: string;
	channel_id: string;
	author_id: string;
	content: string;
	timestamp: number;
	edited_timestamp: number | null;
}

export interface AuthResponse {
	token: string;
	user: User;
}
