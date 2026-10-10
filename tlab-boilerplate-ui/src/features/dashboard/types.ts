export interface DashboardSnapshot {
	sampled_at: string;
	scope: "host";
	cpu: {
		total_usage_percent: number | null;
		cores_usage_percent: number[] | null;
	};
	memory: {
		total_bytes: number;
		available_bytes: number;
	};
	processes_sampled_at: string;
	processes: {
		pid: number;
		name: string;
		memory_bytes: number;
		cpu_usage_percent: number;
	}[];
	application: {
		http_requests_in_flight: number;
		database_pool: {
			max_connections: number;
			size: number;
			idle: number;
		};
	};
}

export interface HistoryPoint {
	timestamp: number;
	cpuPercent: number | null;
	memoryPercent: number;
}
