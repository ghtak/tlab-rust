import { queryOptions } from "@tanstack/react-query";
import { getDashboardSnapshot } from "./api";

export const dashboardSnapshotQuery = queryOptions({
	queryKey: ["dashboardSnapshot"],
	queryFn: getDashboardSnapshot,
	refetchInterval: 5_000,
	refetchIntervalInBackground: false,
	retry: false,
});
