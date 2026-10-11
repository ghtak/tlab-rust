import { enMessages } from "./messages/en";
import { koMessages } from "./messages/ko";

export const defaultLocale = "ko";

export const messages = {
	ko: koMessages,
	en: enMessages,
} as const;

export type AppLocale = keyof typeof messages;
export type MessageOverrides = Partial<
	Record<AppLocale, Record<string, string>>
>;

export function resolveLocale(language: string): AppLocale {
	return language.toLowerCase().split("-")[0] === "en" ? "en" : defaultLocale;
}

export function getMessages(
	locale: AppLocale,
	overrides: MessageOverrides = {},
): Record<string, string> {
	return { ...messages[locale], ...overrides[locale] };
}
