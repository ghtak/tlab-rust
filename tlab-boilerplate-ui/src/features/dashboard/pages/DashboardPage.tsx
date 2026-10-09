import { Link } from "@tanstack/react-router";
import { ArrowUpRight, KeyRound, ShieldCheck, Users } from "lucide-react";
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from "../../../components/ui/card";

const sections = [
	{
		to: "/permissions",
		label: "퍼미션",
		count: "4",
		description: "서비스 접근 권한을 확인합니다.",
		icon: KeyRound,
	},
	{
		to: "/roles",
		label: "롤",
		count: "3",
		description: "퍼미션을 묶은 롤을 확인합니다.",
		icon: ShieldCheck,
	},
	{
		to: "/users",
		label: "사용자",
		count: "4",
		description: "사용자와 연결된 롤을 확인합니다.",
		icon: Users,
	},
] as const;

export function DashboardPage() {
	return (
		<div className="space-y-8">
			<div>
				<p className="mb-3 text-sm text-slate-500">홈 / 대시보드</p>
				<div className="flex flex-wrap items-end justify-between gap-4">
					<div>
						<h1 className="text-3xl font-semibold tracking-tight">대시보드</h1>
						<p className="mt-2 text-sm text-slate-500">
							접근 관리 화면을 둘러보는 시작 페이지입니다.
						</p>
					</div>
					<span className="rounded-full border border-slate-200 bg-white px-3 py-1.5 text-xs font-medium text-slate-500">
						레이아웃 미리보기 · 임시 데이터
					</span>
				</div>
			</div>

			<div className="grid gap-4 md:grid-cols-3">
				{sections.map(({ to, label, count, description, icon: Icon }) => (
					<Link
						key={to}
						to={to}
						className="group block rounded-xl outline-none focus-visible:ring-2 focus-visible:ring-ring"
					>
						<Card className="h-full gap-6 bg-white transition-shadow group-hover:shadow-md">
							<CardHeader>
								<div className="flex items-center justify-between">
									<span className="flex size-10 items-center justify-center rounded-lg bg-slate-100 text-slate-700">
										<Icon className="size-5" aria-hidden="true" />
									</span>
									<ArrowUpRight
										className="size-4 text-slate-400 transition-colors group-hover:text-slate-900"
										aria-hidden="true"
									/>
								</div>
								<CardTitle className="mt-3 text-sm text-slate-600">
									{label}
								</CardTitle>
							</CardHeader>
							<CardContent>
								<p className="text-3xl font-semibold tracking-tight">{count}</p>
								<CardDescription className="mt-2">
									{description}
								</CardDescription>
							</CardContent>
						</Card>
					</Link>
				))}
			</div>

			<Card className="gap-3 bg-white">
				<CardHeader>
					<CardTitle>시작하기</CardTitle>
					<CardDescription>
						좌측 메뉴를 열거나 위 카드를 선택해 각 화면으로 이동할 수 있습니다.
					</CardDescription>
				</CardHeader>
			</Card>
		</div>
	);
}
