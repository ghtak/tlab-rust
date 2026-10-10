import { useQuery } from "@tanstack/react-query";
import { HTTPError } from "ky";
import { useEffect, useState } from "react";
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from "../../../components/ui/card";
import {
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from "../../../components/ui/table";
import { MetricChart } from "../components/MetricChart";
import { dashboardSnapshotQuery } from "../services/queries";
import type { DashboardSnapshot, HistoryPoint } from "../types";

const HISTORY_WINDOW_MS = 30 * 60 * 1_000;

function appendHistory(
	points: HistoryPoint[],
	snapshot: DashboardSnapshot,
): HistoryPoint[] {
	const timestamp = Date.parse(snapshot.sampled_at);
	if (points.at(-1)?.timestamp === timestamp) return points;
	const memoryPercent =
		snapshot.memory.total_bytes > 0
			? (Math.max(
					0,
					snapshot.memory.total_bytes - snapshot.memory.available_bytes,
				) /
					snapshot.memory.total_bytes) *
				100
			: 0;
	return [
		...points.filter(
			(point) => point.timestamp >= timestamp - HISTORY_WINDOW_MS,
		),
		{ timestamp, cpuPercent: snapshot.cpu.total_usage_percent, memoryPercent },
	].slice(-360);
}

export function DashboardPage() {
	const { data, error, isPending, isError } = useQuery(dashboardSnapshotQuery);
	const [history, setHistory] = useState<HistoryPoint[]>([]);

	useEffect(() => {
		if (data) setHistory((points) => appendHistory(points, data));
	}, [data]);

	const memoryUsed = data
		? Math.max(0, data.memory.total_bytes - data.memory.available_bytes)
		: 0;
	const memoryPercent =
		data && data.memory.total_bytes > 0
			? (memoryUsed / data.memory.total_bytes) * 100
			: null;
	const pool = data?.application.database_pool;

	return (
		<div className="space-y-6">
			<div>
				<p className="mb-2 text-sm text-slate-500">홈 / 대시보드</p>
				<div className="flex flex-wrap items-end justify-between gap-3">
					<div>
						<h1 className="text-3xl font-semibold tracking-tight">
							시스템 대시보드
						</h1>
						<p className="mt-2 text-sm text-slate-500">
							호스트 자원과 현재 서버의 요청·DB 풀 상태
						</p>
					</div>
					<p className="text-xs text-slate-500">
						{data
							? `마지막 수집 ${formatTime(data.sampled_at)}`
							: "수집 대기 중"}
					</p>
				</div>
			</div>

			{isError && (
				<p
					role="alert"
					className="rounded-lg border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700"
				>
					{data
						? "최신 데이터를 가져오지 못했습니다. 마지막 수집 값을 표시합니다."
						: error instanceof HTTPError && error.response.status === 503
							? "시스템 정보를 수집하는 중입니다. 잠시 후 자동으로 갱신됩니다."
							: "시스템 정보를 가져오지 못했습니다. 잠시 후 자동으로 다시 시도합니다."}
				</p>
			)}
			{isPending && (
				<p className="text-sm text-slate-500">
					시스템 정보를 불러오는 중입니다.
				</p>
			)}

			{data && (
				<>
					<div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
						<SummaryCard
							title="CPU 사용률"
							value={formatPercent(data.cpu.total_usage_percent)}
							description={`${data.cpu.cores_usage_percent?.length ?? "—"}개 논리 코어 · 호스트`}
						/>
						<SummaryCard
							title="메모리 사용량"
							value={formatBytes(memoryUsed)}
							description={`${formatBytes(data.memory.total_bytes)} 중 ${formatPercent(memoryPercent)} · 호스트`}
						/>
						<SummaryCard
							title="처리 중 HTTP 요청"
							value={String(data.application.http_requests_in_flight)}
							description="현재 이 서버 인스턴스에서 처리 중"
						/>
						<SummaryCard
							title="DB 풀 사용 중"
							value={pool ? String(pool.size - pool.idle) : "—"}
							description={
								pool
									? `열린 연결 ${pool.size} · 유휴 ${pool.idle} · 최대 ${pool.max_connections}`
									: ""
							}
						/>
					</div>

					<div className="grid gap-4 xl:grid-cols-2">
						<Card className="bg-white">
							<CardHeader>
								<CardTitle>CPU 사용률</CardTitle>
								<CardDescription>호스트 전체 · 최근 30분</CardDescription>
							</CardHeader>
							<CardContent>
								<MetricChart
									label="CPU 사용률"
									points={history}
									value={(point) => point.cpuPercent}
									color="#2563eb"
								/>
							</CardContent>
						</Card>
						<Card className="bg-white">
							<CardHeader>
								<CardTitle>메모리 사용률</CardTitle>
								<CardDescription>호스트 전체 · 최근 30분</CardDescription>
							</CardHeader>
							<CardContent>
								<MetricChart
									label="메모리 사용률"
									points={history}
									value={(point) => point.memoryPercent}
									color="#059669"
								/>
							</CardContent>
						</Card>
					</div>

					<Card className="bg-white">
						<CardHeader>
							<CardTitle>코어별 CPU 사용률</CardTitle>
							<CardDescription>논리 코어 기준</CardDescription>
						</CardHeader>
						<CardContent>
							{data.cpu.cores_usage_percent ? (
								<div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
									{Object.entries(data.cpu.cores_usage_percent).map(
										([core, usage]) => (
											<div key={core} className="space-y-1.5">
												<div className="flex justify-between text-sm">
													<span>코어 {Number(core) + 1}</span>
													<span>{formatPercent(usage)}</span>
												</div>
												<div className="h-2 rounded-full bg-slate-100">
													<div
														className="h-2 rounded-full bg-blue-500"
														style={{
															width: `${Math.max(0, Math.min(100, usage))}%`,
														}}
													/>
												</div>
											</div>
										),
									)}
								</div>
							) : (
								<p className="text-sm text-slate-500">
									CPU 사용률 측정 중입니다.
								</p>
							)}
						</CardContent>
					</Card>

					<Card className="bg-white">
						<CardHeader>
							<CardTitle>메모리 사용량 상위 프로세스</CardTitle>
							<CardDescription>
								현재 실행 환경에서 보이는 프로세스 ·{" "}
								{formatTime(data.processes_sampled_at)}
							</CardDescription>
						</CardHeader>
						<CardContent>
							<Table>
								<TableHeader>
									<TableRow>
										<TableHead>PID</TableHead>
										<TableHead>프로세스</TableHead>
										<TableHead className="text-right">메모리</TableHead>
										<TableHead className="text-right">CPU</TableHead>
									</TableRow>
								</TableHeader>
								<TableBody>
									{data.processes.map((process) => (
										<TableRow key={process.pid}>
											<TableCell>{process.pid}</TableCell>
											<TableCell
												className="max-w-60 truncate"
												title={process.name}
											>
												{process.name}
											</TableCell>
											<TableCell className="text-right">
												{formatBytes(process.memory_bytes)}
											</TableCell>
											<TableCell className="text-right">
												{formatPercent(process.cpu_usage_percent)}
											</TableCell>
										</TableRow>
									))}
									{data.processes.length === 0 && (
										<TableRow>
											<TableCell
												colSpan={4}
												className="py-6 text-center text-slate-500"
											>
												표시할 프로세스가 없습니다.
											</TableCell>
										</TableRow>
									)}
								</TableBody>
							</Table>
						</CardContent>
					</Card>
				</>
			)}
		</div>
	);
}

function SummaryCard({
	title,
	value,
	description,
}: {
	title: string;
	value: string;
	description: string;
}) {
	return (
		<Card className="bg-white">
			<CardHeader>
				<CardTitle className="text-sm text-slate-600">{title}</CardTitle>
			</CardHeader>
			<CardContent>
				<p className="text-3xl font-semibold tracking-tight">{value}</p>
				<CardDescription className="mt-2">{description}</CardDescription>
			</CardContent>
		</Card>
	);
}

function formatPercent(value: number | null): string {
	return value === null ? "측정 중" : `${value.toFixed(1)}%`;
}

function formatBytes(value: number): string {
	const units = ["B", "KiB", "MiB", "GiB", "TiB"];
	let size = value;
	let index = 0;
	while (size >= 1024 && index < units.length - 1) {
		size /= 1024;
		index += 1;
	}
	return `${size.toFixed(index === 0 ? 0 : 1)} ${units[index]}`;
}

function formatTime(value: string): string {
	return new Date(value).toLocaleTimeString("ko-KR", {
		hour: "2-digit",
		minute: "2-digit",
		second: "2-digit",
	});
}
