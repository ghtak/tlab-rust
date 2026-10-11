import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import { ThemeProvider, useTheme } from "./ThemeProvider";

afterEach(() => {
	cleanup();
	window.localStorage.clear();
	document.documentElement.classList.remove("dark");
	vi.unstubAllGlobals();
});

test("테마 선택을 저장하고 시스템 설정 변경을 따른다", () => {
	let systemDark = true;
	let onChange: (() => void) | undefined;
	vi.stubGlobal("matchMedia", () => ({
		get matches() {
			return systemDark;
		},
		addEventListener: (_event: string, listener: () => void) => {
			onChange = listener;
		},
		removeEventListener: () => {
			onChange = undefined;
		},
	}));

	function Controls() {
		const { theme, setTheme } = useTheme();
		return (
			<>
				<span>{theme}</span>
				<button type="button" onClick={() => setTheme("light")}>
					Light
				</button>
				<button type="button" onClick={() => setTheme("system")}>
					System
				</button>
			</>
		);
	}

	render(
		<ThemeProvider>
			<Controls />
		</ThemeProvider>,
	);
	expect(document.documentElement.classList.contains("dark")).toBe(true);

	fireEvent.click(screen.getByRole("button", { name: "Light" }));
	expect(window.localStorage.getItem("tlab.theme")).toBe("light");
	expect(document.documentElement.classList.contains("dark")).toBe(false);

	fireEvent.click(screen.getByRole("button", { name: "System" }));
	systemDark = false;
	onChange?.();
	expect(document.documentElement.classList.contains("dark")).toBe(false);
	systemDark = true;
	onChange?.();
	expect(document.documentElement.classList.contains("dark")).toBe(true);
});
