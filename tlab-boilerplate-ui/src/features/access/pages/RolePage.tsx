import { Card, CardHeader, CardTitle } from "../../../components/ui/card";

const roles = [
	{ code: "admin", description: "관리자", permissions: "4개", users: "2명" },
	{
		code: "sales",
		description: "영업 담당자",
		permissions: "1개",
		users: "8명",
	},
	{
		code: "viewer",
		description: "조회 전용",
		permissions: "2개",
		users: "14명",
	},
];

export function RolePage() {
	return (
		<div className="space-y-8">
			<div>
				<p className="mb-3 text-sm text-slate-500">
					접근 관리 / <span className="text-slate-900">롤</span>
				</p>
				<div className="flex flex-wrap items-end justify-between gap-4">
					<div>
						<h1 className="text-3xl font-semibold tracking-tight">롤</h1>
						<p className="mt-2 text-sm text-slate-500">
							사용자에게 부여할 롤과 연결된 퍼미션을 살펴보는 화면입니다.
						</p>
					</div>
					<span className="rounded-full border border-slate-200 bg-white px-3 py-1.5 text-xs font-medium text-slate-500">
						레이아웃 미리보기 · 임시 데이터
					</span>
				</div>
			</div>
			<Card className="gap-0 bg-white py-0 shadow-sm">
				<CardHeader className="flex flex-row items-center justify-between gap-4 border-b border-slate-100 py-4 sm:px-6">
					<div>
						<CardTitle>목록</CardTitle>
						<p className="mt-0.5 text-xs text-slate-500">
							화면 구성을 위한 예시 항목입니다.
						</p>
					</div>
					<span className="text-sm text-slate-500">{roles.length}개 항목</span>
				</CardHeader>
				<div className="overflow-x-auto">
					<table className="w-full min-w-[620px] text-left text-sm">
						<thead className="bg-slate-50 text-xs text-slate-500">
							<tr>
								<th scope="col" className="px-5 py-3 font-medium sm:px-6">
									코드
								</th>
								<th scope="col" className="px-5 py-3 font-medium sm:px-6">
									설명
								</th>
								<th scope="col" className="px-5 py-3 font-medium sm:px-6">
									퍼미션
								</th>
								<th scope="col" className="px-5 py-3 font-medium sm:px-6">
									사용자
								</th>
							</tr>
						</thead>
						<tbody className="divide-y divide-slate-100">
							{roles.map((role) => (
								<tr key={role.code} className="hover:bg-slate-50/70">
									<td className="px-5 py-4 font-medium text-slate-900 sm:px-6">
										{role.code}
									</td>
									<td className="px-5 py-4 text-slate-600 sm:px-6">
										{role.description}
									</td>
									<td className="px-5 py-4 text-slate-600 sm:px-6">
										{role.permissions}
									</td>
									<td className="px-5 py-4 text-slate-600 sm:px-6">
										{role.users}
									</td>
								</tr>
							))}
						</tbody>
					</table>
				</div>
			</Card>
		</div>
	);
}
