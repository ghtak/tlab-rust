import {
	createContext,
	type ReactNode,
	useContext,
	useEffect,
	useState,
} from "react";
import { IntlProvider } from "react-intl";
import {
	type AppLocale,
	defaultLocale,
	getMessages,
	type MessageOverrides,
	resolveLocale,
} from "./messages";

const LOCALE_STORAGE_KEY = "tlab.locale";

const LocaleContext = createContext<{
	locale: AppLocale;
	setLocale: (locale: AppLocale) => void;
} | null>(null);

function initialLocale(): AppLocale {
	const saved = window.localStorage.getItem(LOCALE_STORAGE_KEY);
	return resolveLocale(saved || window.navigator.language);
}

export function AppIntlProvider({
	children,
	overrides,
}: {
	children: ReactNode;
	overrides?: MessageOverrides;
}) {
	const [locale, setCurrentLocale] = useState(initialLocale);

	function setLocale(nextLocale: AppLocale) {
		window.localStorage.setItem(LOCALE_STORAGE_KEY, nextLocale);
		setCurrentLocale(nextLocale);
	}

	useEffect(() => {
		document.documentElement.lang = locale;
	}, [locale]);

	return (
		<LocaleContext.Provider value={{ locale, setLocale }}>
			<IntlProvider
				locale={locale}
				defaultLocale={defaultLocale}
				messages={getMessages(locale, overrides)}
			>
				{children}
			</IntlProvider>
		</LocaleContext.Provider>
	);
}

export function useAppLocale() {
	const context = useContext(LocaleContext);
	if (!context) throw new Error("AppIntlProvider is missing");
	return context;
}
