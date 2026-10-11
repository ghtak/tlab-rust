import { Link } from "@tanstack/react-router";
import { FormattedMessage } from "react-intl";

export function AccessDeniedPage() {
	return (
		<div>
			<h1 className="text-2xl font-semibold">
				<FormattedMessage
					id="auth.denied.title"
					defaultMessage="접근 권한이 없습니다."
				/>
			</h1>
			<p className="mt-3 text-slate-600">
				<FormattedMessage
					id="auth.denied.description"
					defaultMessage="이 화면을 볼 수 있는 권한이 필요합니다."
				/>
			</p>
			<Link to="/" className="mt-6 inline-block text-sm font-medium underline">
				<FormattedMessage
					id="auth.denied.back"
					defaultMessage="대시보드로 이동"
				/>
			</Link>
		</div>
	);
}
