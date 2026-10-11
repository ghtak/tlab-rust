import { useIntl } from "react-intl";
import { type Theme, useTheme } from "../lib/ThemeProvider";

export function ThemeSelect() {
	const intl = useIntl();
	const { theme, setTheme } = useTheme();
	return (
		<select
			aria-label={intl.formatMessage({
				id: "common.theme",
				defaultMessage: "테마",
			})}
			className="rounded-md border border-border bg-background px-2 py-1 text-sm text-foreground"
			value={theme}
			onChange={(event) => setTheme(event.target.value as Theme)}
		>
			<option value="light">
				{intl.formatMessage({ id: "common.themeLight", defaultMessage: "라이트" })}
			</option>
			<option value="dark">
				{intl.formatMessage({ id: "common.themeDark", defaultMessage: "다크" })}
			</option>
			<option value="system">
				{intl.formatMessage({ id: "common.themeSystem", defaultMessage: "시스템" })}
			</option>
		</select>
	);
}
