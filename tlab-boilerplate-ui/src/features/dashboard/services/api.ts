import { api } from "../../../api";
import type { DashboardSnapshot } from "../types";

export async function getDashboardSnapshot(): Promise<DashboardSnapshot> {
	const response = await api
		.get("dashboard/snapshot", { cache: "no-store" })
		.json<{ data: DashboardSnapshot }>();
	return response.data;
}
