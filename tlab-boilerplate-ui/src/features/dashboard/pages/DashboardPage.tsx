import { useQuery } from "@tanstack/react-query";
import { HTTPError } from "ky";
import { useEffect, useState } from "react";
import { FormattedMessage, useIntl } from "react-intl";
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
	const intl = useIntl();
	const percent = (value: number | null) =>
		value === null
			? intl.formatMessage({
					id: "dashboard.measuring",
					defaultMessage: "측정 중",
				})
			: intl.formatNumber(value / 100, {
					style: "percent",
					minimumFractionDigits: 1,
					maximumFractionDigits: 1,
				});
	const bytes = (value: number) => formatBytes(value, intl.locale);
	const time = (value: string) => formatTime(value, intl.locale);
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
				<p className="mb-2 text-sm text-muted-foreground">
					<FormattedMessage
						id="dashboard.breadcrumb"
						defaultMessage="홈 / 대시보드"
					/>
				</p>
				<div className="flex flex-wrap items-end justify-between gap-3">
					<div>
						<h1 className="text-3xl font-semibold tracking-tight">
							<FormattedMessage
								id="dashboard.title"
								defaultMessage="시스템 대시보드"
							/>
						</h1>
						<p className="mt-2 text-sm text-muted-foreground">
							<FormattedMessage
								id="dashboard.description"
								defaultMessage="호스트 자원과 현재 서버의 요청·DB 풀 상태"
							/>
						</p>
					</div>
					<p className="text-xs text-muted-foreground">
						{data ? (
							<FormattedMessage
								id="dashboard.lastSample"
								defaultMessage="마지막 수집 {time}"
								values={{ time: time(data.sampled_at) }}
							/>
						) : (
							<FormattedMessage
								id="dashboard.waiting"
								defaultMessage="수집 대기 중"
							/>
						)}
					</p>
				</div>
			</div>

			{isError && (
				<p
					role="alert"
					className="rounded-lg border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive"
				>
					{data ? (
						<FormattedMessage
							id="dashboard.stale"
							defaultMessage="최신 데이터를 가져오지 못했습니다. 마지막 수집 값을 표시합니다."
						/>
					) : error instanceof HTTPError && error.response.status === 503 ? (
						<FormattedMessage
							id="dashboard.collecting"
							defaultMessage="시스템 정보를 수집하는 중입니다. 잠시 후 자동으로 갱신됩니다."
						/>
					) : (
						<FormattedMessage
							id="dashboard.loadFailed"
							defaultMessage="시스템 정보를 가져오지 못했습니다. 잠시 후 자동으로 다시 시도합니다."
						/>
					)}
				</p>
			)}
			{isPending && (
				<p className="text-sm text-muted-foreground">
					<FormattedMessage
						id="dashboard.loading"
						defaultMessage="시스템 정보를 불러오는 중입니다."
					/>
				</p>
			)}

			{data && (
				<>
					<div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
						<SummaryCard
							title={intl.formatMessage({
								id: "dashboard.cpuUsage",
								defaultMessage: "CPU 사용률",
							})}
							value={percent(data.cpu.total_usage_percent)}
							description={intl.formatMessage(
								{
									id: "dashboard.logicalCores",
									defaultMessage: "{count}개 논리 코어 · 호스트",
								},
								{ count: data.cpu.cores_usage_percent?.length ?? "—" },
							)}
						/>
						<SummaryCard
							title={intl.formatMessage({
								id: "dashboard.memoryUsage",
								defaultMessage: "메모리 사용량",
							})}
							value={bytes(memoryUsed)}
							description={intl.formatMessage(
								{
									id: "dashboard.memoryTotal",
									defaultMessage: "{total} 중 {percent} · 호스트",
								},
								{
									total: bytes(data.memory.total_bytes),
									percent: percent(memoryPercent),
								},
							)}
						/>
						<SummaryCard
							title={intl.formatMessage({
								id: "dashboard.httpRequests",
								defaultMessage: "처리 중 HTTP 요청",
							})}
							value={String(data.application.http_requests_in_flight)}
							description={intl.formatMessage({
								id: "dashboard.currentInstance",
								defaultMessage: "현재 이 서버 인스턴스에서 처리 중",
							})}
						/>
						<SummaryCard
							title={intl.formatMessage({
								id: "dashboard.poolInUse",
								defaultMessage: "DB 풀 사용 중",
							})}
							value={pool ? String(pool.size - pool.idle) : "—"}
							description={
								pool
									? intl.formatMessage(
											{
												id: "dashboard.poolSummary",
												defaultMessage:
													"열린 연결 {size} · 유휴 {idle} · 최대 {max}",
											},
											{
												size: pool.size,
												idle: pool.idle,
												max: pool.max_connections,
											},
										)
									: ""
							}
						/>
					</div>

					<div className="grid gap-4 xl:grid-cols-2">
						<Card className="bg-card">
							<CardHeader>
								<CardTitle>
									<FormattedMessage
										id="dashboard.cpuUsage"
										defaultMessage="CPU 사용률"
									/>
								</CardTitle>
								<CardDescription>
									<FormattedMessage
										id="dashboard.hostRecent"
										defaultMessage="호스트 전체 · 최근 30분"
									/>
								</CardDescription>
							</CardHeader>
							<CardContent>
								<MetricChart
									label={intl.formatMessage({
										id: "dashboard.cpuUsage",
										defaultMessage: "CPU 사용률",
									})}
									points={history}
									value={(point) => point.cpuPercent}
									color="#2563eb"
								/>
							</CardContent>
						</Card>
						<Card className="bg-card">
							<CardHeader>
								<CardTitle>
									<FormattedMessage
										id="dashboard.memoryPercent"
										defaultMessage="메모리 사용률"
									/>
								</CardTitle>
								<CardDescription>
									<FormattedMessage
										id="dashboard.hostRecent"
										defaultMessage="호스트 전체 · 최근 30분"
									/>
								</CardDescription>
							</CardHeader>
							<CardContent>
								<MetricChart
									label={intl.formatMessage({
										id: "dashboard.memoryPercent",
										defaultMessage: "메모리 사용률",
									})}
									points={history}
									value={(point) => point.memoryPercent}
									color="#059669"
								/>
							</CardContent>
						</Card>
					</div>

					<Card className="bg-card">
						<CardHeader>
							<CardTitle>
								<FormattedMessage
									id="dashboard.cpuByCore"
									defaultMessage="코어별 CPU 사용률"
								/>
							</CardTitle>
							<CardDescription>
								<FormattedMessage
									id="dashboard.logicalCoreBasis"
									defaultMessage="논리 코어 기준"
								/>
							</CardDescription>
						</CardHeader>
						<CardContent>
							{data.cpu.cores_usage_percent ? (
								<div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
									{Object.entries(data.cpu.cores_usage_percent).map(
										([core, usage]) => (
											<div key={core} className="space-y-1.5">
												<div className="flex justify-between text-sm">
													<span>
														<FormattedMessage
															id="dashboard.core"
															defaultMessage="코어 {number}"
															values={{ number: Number(core) + 1 }}
														/>
													</span>
													<span>{percent(usage)}</span>
												</div>
												<div className="h-2 rounded-full bg-muted">
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
								<p className="text-sm text-muted-foreground">
									<FormattedMessage
										id="dashboard.measuringCpu"
										defaultMessage="CPU 사용률 측정 중입니다."
									/>
								</p>
							)}
						</CardContent>
					</Card>

					<Card className="bg-card">
						<CardHeader>
							<CardTitle>
								<FormattedMessage
									id="dashboard.topProcesses"
									defaultMessage="메모리 사용량 상위 프로세스"
								/>
							</CardTitle>
							<CardDescription>
								<FormattedMessage
									id="dashboard.processesAt"
									defaultMessage="현재 실행 환경에서 보이는 프로세스 · {time}"
									values={{ time: time(data.processes_sampled_at) }}
								/>
							</CardDescription>
						</CardHeader>
						<CardContent>
							<Table>
								<TableHeader>
									<TableRow>
										<TableHead>PID</TableHead>
										<TableHead>
											<FormattedMessage
												id="dashboard.process"
												defaultMessage="프로세스"
											/>
										</TableHead>
										<TableHead className="text-right">
											<FormattedMessage
												id="dashboard.memory"
												defaultMessage="메모리"
											/>
										</TableHead>
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
												{bytes(process.memory_bytes)}
											</TableCell>
											<TableCell className="text-right">
												{percent(process.cpu_usage_percent)}
											</TableCell>
										</TableRow>
									))}
									{data.processes.length === 0 && (
										<TableRow>
											<TableCell
												colSpan={4}
												className="py-6 text-center text-muted-foreground"
											>
												<FormattedMessage
													id="dashboard.noProcesses"
													defaultMessage="표시할 프로세스가 없습니다."
												/>
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
		<Card className="bg-card">
			<CardHeader>
				<CardTitle className="text-sm text-muted-foreground">{title}</CardTitle>
			</CardHeader>
			<CardContent>
				<p className="text-3xl font-semibold tracking-tight">{value}</p>
				<CardDescription className="mt-2">{description}</CardDescription>
			</CardContent>
		</Card>
	);
}

function formatBytes(value: number, locale: string): string {
	const units = ["B", "KiB", "MiB", "GiB", "TiB"];
	let size = value;
	let index = 0;
	while (size >= 1024 && index < units.length - 1) {
		size /= 1024;
		index += 1;
	}
	return `${new Intl.NumberFormat(locale, { minimumFractionDigits: index === 0 ? 0 : 1, maximumFractionDigits: index === 0 ? 0 : 1 }).format(size)} ${units[index]}`;
}

function formatTime(value: string, locale: string): string {
	return new Date(value).toLocaleTimeString(locale, {
		hour: "2-digit",
		minute: "2-digit",
		second: "2-digit",
	});
}
