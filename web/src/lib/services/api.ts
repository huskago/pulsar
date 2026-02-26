import type { AuthResponse } from '$lib/types/models';

const API_BASE = 'http://localhost:3000';

class ApiService {
	private token: string | null = null;

	setToken(token: string) {
		this.token = token;
	}

	clearToken() {
		this.token = null;
	}

	private async request<T>(endpoint: string, options: RequestInit = {}): Promise<T> {
		const headers: Record<string, string> = {
			'Content-Type': 'application/json',
			...((options.headers as Record<string, string>) || {})
		};

		if (this.token) {
			headers['Authorization'] = `Bearer ${this.token}`;
		}

		const response = await fetch(`${API_BASE}${endpoint}`, {
			...options,
			headers
		});

		if (!response.ok) {
			const error = await response.json().catch(() => ({
				error: 'Unknown error'
			}));
			throw new Error(error.error || `HTTP ${response.status}`);
		}

		return response.json();
	}

	async register(username: string, email: string, password: string): Promise<AuthResponse> {
		const res = await this.request<AuthResponse>('/auth/register', {
			method: 'POST',
			body: JSON.stringify({ username, email, password })
		});
		this.setToken(res.token);
		return res;
	}

	async login(email: string, password: string): Promise<AuthResponse> {
		const res = await this.request<AuthResponse>('/auth/login', {
			method: 'POST',
			body: JSON.stringify({ email, password })
		});
		this.setToken(res.token);
		return res;
	}

	async getMe(): Promise<AuthResponse> {
		return this.request('/users/me');
	}
}

export const api = new ApiService();
