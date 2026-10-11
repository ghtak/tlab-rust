import { useIntl } from "react-intl";
import { useAppLocale } from "../lib/AppIntlProvider";

export function LocaleSelect() {
	const intl = useIntl();
	const { locale, setLocale } = useAppLocale();

	return (
		<select
			aria-label={intl.formatMessage({
				id: "common.language",
				defaultMessage: "언어",
			})}
			className="rounded-md border border-border bg-background text-foreground px-2 py-1 text-sm"
			value={locale}
			onChange={(event) => setLocale(event.target.value as "ko" | "en")}
		>
			<option value="ko">한국어</option>
			<option value="en">English</option>
		</select>
	);
}
