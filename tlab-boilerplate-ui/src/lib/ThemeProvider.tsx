import {
	createContext,
	type ReactNode,
	useContext,
	useEffect,
	useState,
} from "react";

export type Theme = "light" | "dark" | "system";

const THEME_STORAGE_KEY = "tlab.theme";
const ThemeContext = createContext<{
	theme: Theme;
	setTheme: (theme: Theme) => void;
} | null>(null);

function initialTheme(): Theme {
	const saved = window.localStorage.getItem(THEME_STORAGE_KEY);
	return saved === "light" || saved === "dark" ? saved : "system";
}

export function ThemeProvider({ children }: { children: ReactNode }) {
	const [theme, setCurrentTheme] = useState<Theme>(initialTheme);

	useEffect(() => {
		const media = window.matchMedia("(prefers-color-scheme: dark)");
		const applyTheme = () => {
			document.documentElement.classList.toggle(
				"dark",
				theme === "dark" || (theme === "system" && media.matches),
			);
		};
		applyTheme();
		if (theme === "system") {
			media.addEventListener("change", applyTheme);
			return () => media.removeEventListener("change", applyTheme);
		}
	}, [theme]);

	function setTheme(nextTheme: Theme) {
		window.localStorage.setItem(THEME_STORAGE_KEY, nextTheme);
		setCurrentTheme(nextTheme);
	}

	return (
		<ThemeContext.Provider value={{ theme, setTheme }}>
			{children}
		</ThemeContext.Provider>
	);
}

export function useTheme() {
	const context = useContext(ThemeContext);
	if (!context) throw new Error("ThemeProvider is missing");
	return context;
}
