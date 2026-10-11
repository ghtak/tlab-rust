import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { FormattedMessage } from "react-intl";
import { afterEach, expect, test } from "vitest";
import { LocaleSelect } from "../components/LocaleSelect";
import { AppIntlProvider } from "./AppIntlProvider";
import { messages } from "./messages";

afterEach(() => {
	cleanup();
	window.localStorage.clear();
});

test("server overrides take precedence and missing messages use their default", () => {
	window.localStorage.setItem("tlab.locale", "ko");
	render(
		<AppIntlProvider overrides={{ ko: { "auth.login.title": "맞춤 로그인" } }}>
			<span>
				<FormattedMessage id="auth.login.title" defaultMessage="로그인" />
			</span>
			<span>
				<FormattedMessage id="unknown.message" defaultMessage="기본 문구" />
			</span>
		</AppIntlProvider>,
	);

	expect(screen.getByText("맞춤 로그인")).toBeTruthy();
	expect(screen.getByText("기본 문구")).toBeTruthy();
});

test("changing language updates messages and the document language", () => {
	window.localStorage.setItem("tlab.locale", "ko");
	render(
		<AppIntlProvider>
			<LocaleSelect />
			<FormattedMessage id="auth.login.title" defaultMessage="로그인" />
		</AppIntlProvider>,
	);

	fireEvent.change(screen.getByRole("combobox", { name: "언어" }), {
		target: { value: "en" },
	});
	expect(screen.getByText("Log in")).toBeTruthy();
	expect(document.documentElement.lang).toBe("en");
	expect(window.localStorage.getItem("tlab.locale")).toBe("en");
});

test("Korean and English catalogs have the same message IDs", () => {
	expect(Object.keys(messages.en).sort()).toEqual(
		Object.keys(messages.ko).sort(),
	);
});
