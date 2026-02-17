import { writable } from 'svelte/store';

export interface BrowserState {
	currentUrl: string;
	pageContent: string;
	pageTitle: string;
	isLoading: boolean;
	isSecure: boolean;
	viaProxy: boolean;
	circuitId: string | null;
	errorMessage: string;
	history: string[];
	historyIndex: number;
}

const initialState: BrowserState = {
	currentUrl: '',
	pageContent: '',
	pageTitle: 'New Tab',
	isLoading: false,
	isSecure: false,
	viaProxy: false,
	circuitId: null,
	errorMessage: '',
	history: [],
	historyIndex: -1
};

function createBrowserStore() {
	const { subscribe, set, update } = writable<BrowserState>(initialState);

	return {
		subscribe,
		reset: () => set(initialState),

		setLoading: (loading: boolean) => update(s => ({ ...s, isLoading: loading })),
		setError: (error: string) => update(s => ({ ...s, errorMessage: error, isLoading: false })),
		clearPage: () => update(s => ({ ...s, pageContent: '', currentUrl: '', pageTitle: 'New Tab', errorMessage: '' })),
		setCurrentUrl: (url: string) => update(s => ({ ...s, currentUrl: url, isLoading: true })),
		clearUrl: () => update(s => ({ ...s, currentUrl: '', pageContent: '', pageTitle: 'New Tab' })),

		setPageContent: (data: {
			url: string;
			content: string;
			viaProxy: boolean;
			circuitId: string | null;
		}) => update(s => {
			const titleMatch = data.content.match(/<title[^>]*>([^<]*)<\/title>/i);
			let hostname = '';
			try {
				hostname = new URL(data.url).hostname;
			} catch {}
			const pageTitle = titleMatch ? titleMatch[1] : hostname;

			// Add to history if different
			let newHistory = s.history;
			let newIndex = s.historyIndex;
			if (s.history[s.historyIndex] !== data.url) {
				newHistory = [...s.history.slice(0, s.historyIndex + 1), data.url];
				newIndex = newHistory.length - 1;
			}

			return {
				...s,
				currentUrl: data.url,
				pageContent: data.content,
				pageTitle,
				viaProxy: data.viaProxy,
				isSecure: data.viaProxy,
				circuitId: data.circuitId,
				errorMessage: '',
				isLoading: false,
				history: newHistory,
				historyIndex: newIndex
			};
		}),

		goBack: () => update(s => {
			if (s.historyIndex > 0) {
				return { ...s, historyIndex: s.historyIndex - 1 };
			}
			return s;
		}),

		goForward: () => update(s => {
			if (s.historyIndex < s.history.length - 1) {
				return { ...s, historyIndex: s.historyIndex + 1 };
			}
			return s;
		}),

		getHistoryUrl: (state: BrowserState) => {
			if (state.historyIndex >= 0 && state.historyIndex < state.history.length) {
				return state.history[state.historyIndex];
			}
			return null;
		}
	};
}

export const browserStore = createBrowserStore();
