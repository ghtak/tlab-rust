import type { HistoryPoint } from "../types";

interface MetricChartProps {
	label: string;
	points: HistoryPoint[];
	value: (point: HistoryPoint) => number | null;
	color: string;
}

export function MetricChart({ label, points, value, color }: MetricChartProps) {
	const valid = points.filter((point) => value(point) !== null);
	const first = valid[0]?.timestamp ?? 0;
	const last = valid.at(-1)?.timestamp ?? first;
	const duration = Math.max(last - first, 1);
	const path = valid
		.map((point, index) => {
			const x = 8 + ((point.timestamp - first) / duration) * 584;
			const y =
				122 - (Math.max(0, Math.min(100, value(point) ?? 0)) / 100) * 114;
			return `${index === 0 ? "M" : "L"}${x.toFixed(1)} ${y.toFixed(1)}`;
		})
		.join(" ");

	return (
		<div>
			<div className="h-36 rounded-lg bg-slate-50">
				{valid.length < 2 ? (
					<p className="flex h-full items-center justify-center text-sm text-slate-500">
						그래프 데이터를 모으는 중입니다.
					</p>
				) : (
					<svg
						viewBox="0 0 600 130"
						preserveAspectRatio="none"
						className="h-full w-full"
						role="img"
						aria-label={`${label} 최근 30분 추이`}
					>
						<path
							d="M8 8 H592 M8 65 H592 M8 122 H592"
							stroke="#e2e8f0"
							fill="none"
						/>
						<path
							d={path}
							stroke={color}
							strokeWidth="2.5"
							fill="none"
							vectorEffect="non-scaling-stroke"
						/>
					</svg>
				)}
			</div>
			<div className="mt-2 flex justify-between text-xs text-slate-500">
				<span>{valid.length > 0 ? formatTime(first) : "—"}</span>
				<span>0–100%</span>
				<span>{valid.length > 0 ? formatTime(last) : "—"}</span>
			</div>
		</div>
	);
}

function formatTime(timestamp: number): string {
	return new Date(timestamp).toLocaleTimeString("ko-KR", {
		hour: "2-digit",
		minute: "2-digit",
	});
}
