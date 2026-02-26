import type { User } from '$lib/types/models';
import { api } from '$lib/services/api';
import { gateway } from '$lib/services/gateway.svelte';

let user: User | null = $state(null);
let token: string | null = $state(null);
let loading = $state(false);
let error: string | null = $state(null);

function loadFromStorage() {
	const saved = localStorage.getItem('pulsar_token');
	const savedUser = localStorage.getItem('pulsar_user');
	if (saved && savedUser) {
		token = saved;
		user = JSON.parse(savedUser);
		api.setToken(saved);
		gateway.connect(saved);
	}
}

async function register(username: string, email: string, password: string) {
	loading = true;
	error = null;
	try {
		const res = await api.register(username, email, password);
		user = res.user;
		token = res.token;
		localStorage.setItem('pulsar_token', res.token);
		localStorage.setItem('pulsar_user', JSON.stringify(res.user));
		gateway.connect(res.token);
	} catch (e) {
		error = e instanceof Error ? e.message : 'Registration failed';
		throw e;
	} finally {
		loading = false;
	}
}

async function login(email: string, password: string) {
	loading = true;
	error = null;
	try {
		const res = await api.login(email, password);
		user = res.user;
		token = res.token;
		localStorage.setItem('pulsar_token', res.token);
		localStorage.setItem('pulsar_user', JSON.stringify(res.user));
		gateway.connect(res.token);
	} catch (e) {
		error = e instanceof Error ? e.message : 'Login failed';
		throw e;
	} finally {
		loading = false;
	}
}

function logout() {
	user = null;
	token = null;
	localStorage.removeItem('pulsar_token');
	localStorage.removeItem('pulsar_user');
	api.clearToken();
	gateway.disconnect();
}

export const auth = {
	get user() { return user; },
	get token() { return token; },
	get loading() { return loading; },
	get error() { return error; },
	loadFromStorage,
	register,
	login,
	logout
};