import { writable } from 'svelte/store';
// Simplified auth store for debugging SSR issues

export interface User {
  id: string;
  email: string;
}

export interface AuthTokens {
  access_token: string;
  refresh_token: string;
  expires_in: number;
  expires_at: number;
}

export interface AuthState {
  user: User | null;
  tokens: AuthTokens | null;
  isAuthenticated: boolean;
  isLoading: boolean;
}

// Khởi tạo state từ localStorage nếu có (chỉ trong browser)
const storedTokens = typeof window !== 'undefined' ? localStorage.getItem('auth_tokens') : null;
const storedUser = typeof window !== 'undefined' ? localStorage.getItem('auth_user') : null;

let initialTokens: AuthTokens | null = null;
let initialUser: User | null = null;

if (typeof window !== 'undefined') {
  if (storedTokens && storedUser) {
    try {
      const tokens = JSON.parse(storedTokens) as AuthTokens;
      const user = JSON.parse(storedUser) as User;

      // Kiểm tra token có hết hạn không
      if (tokens.expires_at > Date.now()) {
        initialTokens = tokens;
        initialUser = user;
      } else {
        // Token hết hạn, xóa khỏi localStorage
        localStorage.removeItem('auth_tokens');
        localStorage.removeItem('auth_user');
      }
    } catch (e) {
      console.warn('Invalid stored auth data:', e);
      localStorage.removeItem('auth_tokens');
      localStorage.removeItem('auth_user');
    }
  }
}

export const authStore = writable<AuthState>({
  user: initialUser,
  tokens: initialTokens,
  isAuthenticated: !!initialUser,
  isLoading: false,
});

export const authActions = {
  async login(email: string, password: string, rememberMe: boolean = false) {
    authStore.update(state => ({ ...state, isLoading: true }));

    try {
      const response = await fetch('/auth/login', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ email, password }),
      });

      if (!response.ok) {
        const errorData = await response.json().catch(() => ({ message: 'Login failed' }));
        throw new Error(errorData.message || `HTTP ${response.status}`);
      }

      const data = await response.json();

      const tokens: AuthTokens = {
        access_token: data.access_token,
        refresh_token: data.refresh_token,
        expires_in: data.expires_in,
        expires_at: Date.now() + (data.expires_in * 1000),
      };

      const user: User = {
        id: data.user_id,
        email: data.email,
      };

      // Lưu vào localStorage (chỉ trong browser)
      if (typeof window !== 'undefined') {
        localStorage.setItem('auth_tokens', JSON.stringify(tokens));
        localStorage.setItem('auth_user', JSON.stringify(user));
      }

      authStore.set({
        user,
        tokens,
        isAuthenticated: true,
        isLoading: false,
      });

      return { success: true };
    } catch (error) {
      authStore.update(state => ({
        ...state,
        isLoading: false
      }));
      return { success: false, error: error.message };
    }
  },

  async refreshToken() {
    authStore.update(state => {
      if (!state.tokens) return state;

      return {
        ...state,
        isLoading: true,
      };
    });

    try {
      const state = await new Promise<AuthState>((resolve) => {
        const unsubscribe = authStore.subscribe(s => resolve(s));
        unsubscribe();
      });

      if (!state.tokens) {
        throw new Error('No tokens available');
      }

      const response = await fetch('/auth/refresh', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          refresh_token: state.tokens?.refresh_token,
        }),
      });

      if (!response.ok) {
        throw new Error('Token refresh failed');
      }

      const data = await response.json();

      const tokens: AuthTokens = {
        access_token: data.access_token,
        refresh_token: data.refresh_token,
        expires_in: data.expires_in,
        expires_at: Date.now() + (data.expires_in * 1000),
      };

      // Lưu vào localStorage (chỉ trong browser)
      if (typeof window !== 'undefined') {
        localStorage.setItem('auth_tokens', JSON.stringify(tokens));
      }

      authStore.update(state => ({
        ...state,
        tokens,
        isLoading: false,
      }));

      return { success: true };
    } catch (error) {
      authStore.update(state => ({
        ...state,
        user: null,
        tokens: null,
        isAuthenticated: false,
        isLoading: false,
      }));

      // Xóa token khỏi localStorage nếu refresh thất bại
      if (typeof window !== 'undefined') {
        localStorage.removeItem('auth_tokens');
        localStorage.removeItem('auth_user');
      }

      return { success: false, error: error.message };
    }
  },

  logout() {
    if (typeof window !== 'undefined') {
      localStorage.removeItem('auth_tokens');
      localStorage.removeItem('auth_user');
    }

    authStore.set({
      user: null,
      tokens: null,
      isAuthenticated: false,
      isLoading: false,
    });
  },

  getAuthHeaders(): Record<string, string> {
    let headers: Record<string, string> = {};

    authStore.subscribe(state => {
      if (state.tokens?.access_token) {
        headers['Authorization'] = `Bearer ${state.tokens.access_token}`;
      }
    });

    return headers;
  },
};

// Auto refresh token trước khi hết hạn 5 phút (chỉ trong browser)
if (typeof window !== 'undefined' && initialTokens) {
  const timeUntilExpiry = initialTokens.expires_at - Date.now() - (5 * 60 * 1000); // 5 minutes before expiry

  if (timeUntilExpiry > 0) {
    setTimeout(() => {
      authActions.refreshToken();
    }, timeUntilExpiry);
  }
}
