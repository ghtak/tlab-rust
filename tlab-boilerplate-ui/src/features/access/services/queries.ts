import { queryOptions } from "@tanstack/react-query";
import type { PermissionListParams } from "../types";
import { getPermissions } from "./api";

export function permissionsQuery(params: PermissionListParams) {
	return queryOptions({
		queryKey: ["permissions", params],
		queryFn: () => getPermissions(params),
		retry: false,
	});
}
