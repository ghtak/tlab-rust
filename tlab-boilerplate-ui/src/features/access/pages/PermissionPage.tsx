import { Card, CardHeader, CardTitle } from "../../../components/ui/card";

const permissions = [
	{ code: "user:manage", description: "사용자 관리", roles: "admin" },
	{ code: "role:manage", description: "롤 관리", roles: "admin" },
	{ code: "permission:manage", description: "퍼미션 관리", roles: "admin" },
	{ code: "file:manage", description: "파일 관리", roles: "admin, sales" },
];

export function PermissionPage() {
	return (
		<div className="space-y-8">
			<div>
				<p className="mb-3 text-sm text-slate-500">
					접근 관리 / <span className="text-slate-900">퍼미션</span>
				</p>
				<div className="flex flex-wrap items-end justify-between gap-4">
					<div>
						<h1 className="text-3xl font-semibold tracking-tight">퍼미션</h1>
						<p className="mt-2 text-sm text-slate-500">
							서비스에서 사용하는 퍼미션을 한눈에 살펴보는 화면입니다.
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
					<span className="text-sm text-slate-500">
						{permissions.length}개 항목
					</span>
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
									연결된 롤
								</th>
							</tr>
						</thead>
						<tbody className="divide-y divide-slate-100">
							{permissions.map((permission) => (
								<tr key={permission.code} className="hover:bg-slate-50/70">
									<td className="px-5 py-4 font-medium text-slate-900 sm:px-6">
										{permission.code}
									</td>
									<td className="px-5 py-4 text-slate-600 sm:px-6">
										{permission.description}
									</td>
									<td className="px-5 py-4 text-slate-600 sm:px-6">
										{permission.roles}
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
